// SPDX-License-Identifier: MPL-2.0
//!454ED0/42D050 body -> raw BGRA thumbnail -> globals -> read flags.
//!Each file has the shared storage provider's atomic replacement boundary;
//!native code has no transaction spanning those files and neither do we.
use super::*;
use crysirror_core::image_processing::ImageResource;
pub(super) struct NativeSave {
    capture: Option<ImageJob>,
    reduction: Option<ImageJob>,
    bytes: Option<Vec<u8>>,
    path: VfsPath,
    globals: Option<Vec<u8>>,
    read_flags: Option<Vec<u8>>,
    writer: Option<Task<()>>,
    resume: Option<Box<Wait>>,
    command: bool,
    slot: i32,
    snapshot: crate::native_storage::slot::Slot,
    confirm: bool,
    dialog: bool,
}
impl SugarPotVm {
    pub(super) fn start_native_save(
        &mut self,
        h: &mut dyn HostServices,
        slot: i32,
        command: bool,
    ) -> Result<()> {
        ensure!(
            self.native_save.is_none(),
            "SugarPot overlapping native save"
        );
        //Sample at the real native safe point, before swapping its wait or
        //starting capture. The writer never invents a PC from displayed text.
        let snapshot = self.native_slot()?;
        let bytes = snapshot.encode()?;
        let globals = crate::native_storage::encode_globals(
            &self
                .globals
                .iter()
                .map(|(name, value)| {
                    Ok(crate::native_storage::Binding {
                        name: name.clone(),
                        value: stored_value(value)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        )?;
        let read_flags = crate::native_storage::encode_read_flags(&self.read_flags)?;
        let path = crate::native_storage::slot_path(slot)?;
        let capture =
            h.begin_layer_prefix_image_over(self.client, [1024, 600], [9000, 0], [0, 0, 0, 255])?;
        let resume = self.wait.take().map(Box::new);
        self.native_save = Some(NativeSave {
            capture: Some(capture),
            reduction: None,
            bytes: Some(bytes),
            path,
            globals: Some(globals),
            read_flags: Some(read_flags),
            writer: None,
            resume,
            command,
            slot,
            snapshot,
            confirm: self.slot_menu.is_some(),
            dialog: false,
        });
        self.wait = Some(Wait::OriginalSave);
        Ok(())
    }
    pub(super) fn poll_native_save(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<Option<Box<Wait>>> {
        let mut save = self
            .native_save
            .take()
            .context("SugarPot missing native save continuation")?;
        let mut cancelled = false;
        let done = (|| -> Result<bool> {
            if save.confirm {
                if !save.dialog {
                    self.submit(h,vec![],vec![json!({"op":"open_dialog","dialog":{"id":65401,"title":"確認","message":"セーブします。よろしいですか？","buttons":[{"id":1,"label":"はい"},{"id":2,"label":"いいえ"}],"default":1,"cancel":2,"font":self.config.font}})])?;
                    save.dialog = true;
                    input.buttons_pressed = 0;
                    return Ok(false);
                }
                let events: Vec<Json> = serde_json::from_slice(
                    &h.query_engine(self.client, EngineQuery::DialogEvents)?,
                )?;
                let Some(event) = events.iter().find(|e| {
                    e["kind"] == "dialog_result"
                        && e["widget"]["id"] == 65401
                        && e["widget"]["owner"] == self.client.0
                }) else {
                    return Ok(false);
                };
                input.buttons_pressed = 0;
                if event["value"].as_f64() != Some(1.) {
                    cancelled = true;
                    return Ok(true);
                }
                save.confirm = false;
            }
            if let Some(job) = &mut save.capture {
                let image: Arc<dyn ImageResource> = match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot save capture: {e}"),
                    ImageProcessState::Ready(image) => image,
                };
                //Native512x300 thumbnail selects40FF10 for this exact2:1
                //ratio: floor(sum of2x2 RGB/4), stored destination alpha0.
                //Shared block average + point reduction expresses that profile.
                ensure!(
                    image.size() == [1024, 600],
                    "SugarPot save capture dimensions"
                );
                save.reduction = Some(
                    h.image_metadata(self.client)?.begin_process(
                        ImageInput::Resource(image),
                        serde_json::to_vec(&vec![
                            crysirror_image::effects::ImageEffect::BlockAverage { cell: [2, 2] },
                            crysirror_image::effects::ImageEffect::ResizeByte {
                                spec: crysirror_image::resize_byte::ResizeByte128 {
                                    size: [Some(512), Some(300)],
                                    sampling:
                                        crysirror_image::resize_byte::ByteResizeSampling::Point,
                                },
                            },
                            crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 0 },
                        ])?
                        .into(),
                    )?,
                );
                save.capture = None;
            }
            if let Some(job) = &mut save.reduction {
                let image = match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot save thumbnail: {e}"),
                    ImageProcessState::Ready(image) => image,
                };
                ensure!(image.size() == [512, 300], "SugarPot thumbnail dimensions");
                let rgba = image.rgba();
                ensure!(
                    rgba.len() == 512 * 300 * 4,
                    "SugarPot thumbnail byte layout"
                );
                let storage = self.storage.as_ref().unwrap().clone();
                let path = save.path.clone();
                let thumbnail = VfsPath::new(
                    Mount::Save,
                    &format!(
                        "{}c.dat",
                        path.relative()
                            .strip_suffix(".dat")
                            .context("SugarPot slot extension")?
                    ),
                )?;
                let bytes = save.bytes.take().unwrap();
                let globals = save.globals.take().unwrap();
                let read_flags = save.read_flags.take().unwrap();
                save.writer = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
                    //Format encoding only: original thumbnail is headerless,
                    //top-down BGRA; it is not another image effect/renderer.
                    let bgra = crysirror_image::packed_pixels::encode(
                        [512, 300],
                        &rgba,
                        crysirror_core::image_processing::PackedPixelLayout::Bgra8,
                        &|| cancel.check(),
                    )?;
                    crate::native_storage::write_file(storage.clone(), &path, &bytes, &cancel)?;
                    crate::native_storage::write_file(storage.clone(), &thumbnail, &bgra, &cancel)?;
                    crate::native_storage::write_file(
                        storage.clone(),
                        &VfsPath::new(Mount::Save, "global.dat")?,
                        &globals,
                        &cancel,
                    )?;
                    crate::native_storage::write_file(
                        storage,
                        &VfsPath::new(Mount::Save, "readal.dat")?,
                        &read_flags,
                        &cancel,
                    )?;
                    Ok(())
                })?);
                save.reduction = None;
            }
            Ok(save
                .writer
                .as_mut()
                .context("SugarPot save writer missing")?
                .try_take()?
                .is_some())
        })();
        match done {
            Ok(true) => {
                self.save_command_done = save.command;
                if !cancelled {
                    self.refresh_slot_after_save(save.slot as usize, save.snapshot);
                }
                Ok(save.resume)
            }
            Ok(false) => {
                self.native_save = Some(save);
                Ok(Some(Box::new(Wait::OriginalSave)))
            }
            Err(error) => {
                self.native_save = Some(save);
                Err(error)
            }
        }
    }
}
