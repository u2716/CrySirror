// SPDX-License-Identifier: MPL-2.0
//! SetTone 4130E0/456D70/46FBF0. The first change keeps a bitmap backup;
//! subsequent changes operate on current pixels, and NULL restores that backup.
//! 46E7B0 builds independently truncated 77B/151G/28R lookup tables.
use super::native_bitmap::NativeBitmap;
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};

struct Item {
    id: u32,
    read: Option<NativeBitmap>,
    input: Option<Arc<dyn ImageResource>>,
    job: Option<ImageJob>,
    output: Option<Arc<dyn ImageResource>>,
}
pub(super) struct Preparing {
    mode: u32,
    items: Vec<Item>,
}
fn effects(mode: u32) -> Result<Arc<[u8]>> {
    let luma = json!({"kind":"fixed_luma","spec":{"coefficients":[28,151,77],
        "groups":[0,1,2],"fractional_bits":8,"amount256":256}});
    let lift = |channel: u8, mask: u8, add: u32| {
        json!({"kind":"channel_lookup",
        "channel":channel,"write_mask":mask,"table":(0..256u32).map(|v|(v+add).min(255)).collect::<Vec<_>>()})
    };
    let list = match mode {
        759 => vec![luma],
        370 => vec![luma, lift(0, 3, 30)],
        750 => vec![luma, lift(2, 4, 80), lift(1, 2, 20)],
        598 => vec![json!({"kind":"invert"})],
        _ => vec![],
    };
    Ok(Arc::from(serde_json::to_vec(&list)?))
}
impl N2SystemVm {
    pub(super) fn publish_native_bitmap(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        image: Arc<dyn ImageResource>,
    ) -> Result<()> {
        if let Some(mut spec) = self.surfaces.get(&id).cloned() {
            let uploaded = host.publish_image_resource(self.client, id, image)?;
            let path = uploaded.path.relative().to_owned();
            spec.initial = Some(path.clone());
            self.submit(
                host,
                vec![json!({"op":"create_render_target","id":id,"spec":spec})],
                vec![],
            )?;
            self.surfaces.insert(id, spec);
            if let Some(old) = self.captured_images.insert(id, path) {
                host.release_image_resource(self.client, &old)?;
            }
        } else {
            if self.effects.contains_key(&id) {
                self.submit(
                    host,
                    vec![json!({"op":"set_paint","id":id,"paint":{"kind":"group"}})],
                    vec![],
                )?;
                self.effects.get_mut(&id).unwrap().rebuild_view = true;
            }
            self.publish_capture(host, id, image)?;
        }
        Ok(())
    }
    pub(super) fn native_tone(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(args.len() == 2, "N2System SetTone signature");
        if state.tone.is_none() {
            let selector = args.string(1)?;
            let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(&selector);
            ensure!(!invalid && bytes.len() <= 4096, "N2System tone selector");
            let mode: u32 = bytes
                .iter()
                .map(|&v| u32::from(if v >= 0x5a { v - 32 } else { v }))
                .sum();
            // Unknown hashes explicitly leave the native bitmap unchanged.
            if !matches!(mode, 315 | 759 | 370 | 750 | 598) {
                self.machine.complete(&mut state.thread, Value::Null)?;
                return Ok(true);
            }
            let mut targets = BTreeSet::new();
            for root in self.query(current, &args.string(0)?)? {
                targets.extend(self.entities.bitmap_targets(root));
            }
            let mut items = Vec::new();
            for id in targets {
                if !matches!(
                    self.entities.nodes[&id].kind,
                    Kind::Visual | Kind::Image { .. }
                ) {
                    continue;
                }
                let layer = if let Kind::Image { paint, size } = &self.entities.nodes[&id].kind {
                    serde_json::from_value(
                        json!({"id":id,"rect":[0,0,size[0],size[1]],"paint":paint}),
                    )?
                } else {
                    serde_json::from_value(self.layer(host, id)?)?
                };
                if mode == 315 {
                    if let Some(image) = self.tone_backups.get(&id) {
                        items.push(Item {
                            id,
                            read: None,
                            input: None,
                            job: None,
                            output: Some(image.clone()),
                        });
                    }
                } else {
                    let Some(read) = self.read_native_bitmap_layer(host, id, &layer)? else {
                        return Ok(false);
                    };
                    items.push(Item {
                        id,
                        read: Some(read),
                        input: None,
                        job: None,
                        output: None,
                    });
                }
            }
            state.tone = Some(Preparing { mode, items });
        }
        let preparation = state.tone.as_mut().unwrap();
        let bytes = |i: &Arc<dyn ImageResource>| {
            let [w, h] = i.size();
            u64::from(w) * u64::from(h) * 4
        };
        let mut live = self.tone_backups.values().map(bytes).sum::<u64>()
            + preparation
                .items
                .iter()
                .filter_map(|i| i.input.as_ref())
                .map(|i| 2 * bytes(i))
                .sum::<u64>();
        let budget = if cfg!(target_os = "vita") {
            24 * 1024 * 1024
        } else {
            96 * 1024 * 1024
        };
        for item in &mut preparation.items {
            if item.output.is_some() {
                continue;
            }
            if item.input.is_none() {
                item.input = item.read.as_mut().unwrap().poll(
                    self.client,
                    self.pool.as_ref().unwrap(),
                    self.images.as_ref().unwrap(),
                )?;
                if item.input.is_none() {
                    return Ok(false);
                }
                item.read = None;
                live += 2 * bytes(item.input.as_ref().unwrap());
            }
            ensure!(live <= budget, "N2System tone live byte budget");
            if item.job.is_none() {
                match self.images.as_ref().unwrap().begin_process(
                    ImageInput::Resource(item.input.as_ref().unwrap().clone()),
                    effects(preparation.mode)?,
                ) {
                    Ok(job) => item.job = Some(job),
                    Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                    Err(e) => return Err(e.into()),
                }
            }
            match item.job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("N2System SetTone: {e}"),
                ImageProcessState::Ready(image) => item.output = Some(image),
            }
        }
        let retained: u64 = self
            .tone_backups
            .values()
            .map(|i| {
                let [w, h] = i.size();
                u64::from(w) * u64::from(h) * 4
            })
            .sum();
        let added: u64 = preparation
            .items
            .iter()
            .filter(|i| !self.tone_backups.contains_key(&i.id))
            .filter_map(|i| i.input.as_ref())
            .map(|i| {
                let [w, h] = i.size();
                u64::from(w) * u64::from(h) * 4
            })
            .sum();
        ensure!(
            retained + added
                <= if cfg!(target_os = "vita") {
                    24 * 1024 * 1024
                } else {
                    96 * 1024 * 1024
                },
            "N2System tone backup budget"
        );
        let preparation = state.tone.take().unwrap();
        for item in preparation.items {
            if !self.entities.nodes.contains_key(&item.id) {
                continue;
            }
            if let Some(input) = item.input {
                self.tone_backups.entry(item.id).or_insert(input);
            }
            self.publish_native_bitmap(host, item.id, item.output.unwrap())?;
            if preparation.mode == 315 {
                self.tone_backups.remove(&item.id);
            }
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}
