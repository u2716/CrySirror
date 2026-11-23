// SPDX-License-Identifier: MPL-2.0
//! SetComposite compatibility policy. Trial 413410/44B430/457730 stores
//! incrementing process addresses into a quarter-width destination loop; it
//! does not read the source pixels. Reproducing that would depend on native
//! process memory. This portable extension explicitly copies clipped RGBA.
//! Pixel composition itself belongs to the shared immutable image worker.
use super::native_bitmap::NativeBitmap;
use super::*;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState, ImageResource,
};
struct Target {
    id: u32,
    read: NativeBitmap,
    input: Option<Arc<dyn ImageResource>>,
    job: Option<ImageJob>,
    output: Option<Arc<dyn ImageResource>>,
}
pub(super) struct Preparing {
    source: NativeBitmap,
    image: Option<Arc<dyn ImageResource>>,
    targets: Vec<Target>,
    destination: [i32; 2],
}
impl N2SystemVm {
    pub(super) fn native_composite(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(args.len() == 4, "N2System SetComposite signature");
        if state.composite.is_none() {
            let destination = [
                i32::try_from(args.integer(1)?)?,
                i32::try_from(args.integer(2)?)?,
            ];
            ensure!(
                destination.iter().all(|v| v.unsigned_abs() <= 1 << 20),
                "N2System composite coordinate bounds"
            );
            let Some(source) = self.read_native_bitmap(host, current, &args.string(3)?)? else {
                return Ok(false);
            };
            let mut ids = BTreeSet::new();
            for root in self.query(current, &args.string(0)?)? {
                ids.extend(self.entities.bitmap_targets(root));
            }
            let mut targets = Vec::new();
            for id in ids {
                let layer = match &self.entities.nodes[&id].kind {
                    Kind::Image { paint, size } => serde_json::from_value(
                        json!({"id":id,"rect":[0,0,size[0],size[1]],"paint":paint}),
                    )?,
                    Kind::Visual => serde_json::from_value(self.layer(host, id)?)?,
                    _ => continue,
                };
                let Some(read) = self.read_native_bitmap_layer(host, id, &layer)? else {
                    return Ok(false);
                };
                targets.push(Target {
                    id,
                    read,
                    input: None,
                    job: None,
                    output: None,
                });
            }
            state.composite = Some(Preparing {
                source,
                image: None,
                targets,
                destination,
            });
        }
        let p = state.composite.as_mut().unwrap();
        if p.image.is_none() {
            p.image = p.source.poll(
                self.client,
                self.pool.as_ref().unwrap(),
                self.images.as_ref().unwrap(),
            )?;
            if p.image.is_none() {
                return Ok(false);
            }
        }
        let source = p.image.as_ref().unwrap();
        let [sw, sh] = source.size();
        let bytes = |i: &Arc<dyn ImageResource>| {
            let [w, h] = i.size();
            u64::from(w) * u64::from(h) * 4
        };
        let mut live = bytes(source)
            + p.targets
                .iter()
                .filter_map(|t| t.input.as_ref())
                .map(|i| 2 * bytes(i))
                .sum::<u64>();
        let budget = if cfg!(target_os = "vita") {
            24 * 1024 * 1024
        } else {
            96 * 1024 * 1024
        };
        for target in &mut p.targets {
            if target.output.is_some() {
                continue;
            }
            if target.input.is_none() {
                target.input = target.read.poll(
                    self.client,
                    self.pool.as_ref().unwrap(),
                    self.images.as_ref().unwrap(),
                )?;
                if target.input.is_none() {
                    return Ok(false);
                }
                live += 2 * bytes(target.input.as_ref().unwrap());
            }
            ensure!(live <= budget, "N2System composite live byte budget");
            if target.job.is_none() {
                let input = ImageInput::Composite {
                    base: Box::new(ImageInput::Resource(target.input.as_ref().unwrap().clone())),
                    layers: vec![ImageCompositeLayer {
                        input: Box::new(ImageInput::Resource(source.clone())),
                        source: [0, 0, sw as i32, sh as i32],
                        destination: p.destination,
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
                    Ok(job) => target.job = Some(job),
                    Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                    Err(e) => return Err(e.into()),
                }
            }
            match target.job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("N2System SetComposite: {e}"),
                ImageProcessState::Ready(image) => target.output = Some(image),
            }
        }
        for target in state.composite.take().unwrap().targets {
            if self.entities.nodes.contains_key(&target.id) {
                self.publish_native_bitmap(host, target.id, target.output.unwrap())?;
            }
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}
