//!4375E0/40B670 save/load row policy. Original bodies and pinned metadata
//!are read on workers; fonts, image leases, retained controls and hit testing
//!remain shared engine services. This path does not perform a slot restore.
use super::*;
use crate::machine::text_ops::panel::{PanelDeclarations, PanelRow, PanelText};
use crate::native_save::{BODY_BYTES, NativeSaveBody, SlotKind};
pub(super) struct SelectionCommit {
    mode: u8,
    page: i32,
    declarations: PanelDeclarations,
    result: PanelDeclarations,
}
pub(super) struct SelectionWorker {
    mode: u8,
    page: i32,
    count: usize,
    receiver: Receiver<std::result::Result<Vec<Option<NativeSaveBody>>, String>>,
}
fn part(slot: u8, position: [i32; 2], crop: [i16; 4], state: i16) -> Result<ImagePart> {
    Ok(ImagePart {
        slot,
        position,
        source: [
            crop[0],
            crop[1],
            crop[2]
                .checked_sub(crop[0])
                .context("Circus selection image width")?,
            crop[3]
                .checked_sub(crop[1])
                .context("Circus selection image height")?,
        ],
        state,
        auxiliary_flag: false,
        resource_binding: None,
        binding_declared: false,
    })
}
impl Bridge<'_> {
    pub(in crate::adapter) fn rebuild_native_selection(
        &mut self,
        mode: u8,
        page: i32,
        declarations: &PanelDeclarations,
    ) -> Result<Option<SystemReply>> {
        //4375E0 publication owns the exact slot-read result until all shared
        //images/UI operations ACK. Retrying must not reread a changing slot.
        if let Some(commit) = self.services.system.selection_commit.as_ref() {
            ensure!(
                commit.mode == mode && commit.page == page && commit.declarations == *declarations,
                "Circus selection publication owner"
            );
            let result = commit.result.clone();
            if !self.refresh_panel_template(&result)? {
                return Ok(None);
            }
            self.services.system.selection_commit = None;
            return Ok(Some(SystemReply::Selection(result)));
        }
        if !matches!(mode, 3 | 4) {
            return Ok(Some(SystemReply::Selection(declarations.clone())));
        }
        let count = declarations.rectangles.len();
        if count == 0 {
            return Ok(Some(SystemReply::Selection(declarations.clone())));
        }
        ensure!(count <= 224, "Circus selection row count");
        let Some(metadata) = self.system_metadata()? else {
            return Ok(None);
        };
        let bodies = if let Some(worker) = &self.services.system.selection_worker {
            ensure!(
                worker.mode == mode && worker.page == page && worker.count == count,
                "Circus selection worker owner"
            );
            match worker.receiver.try_recv() {
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => bail!("Circus selection worker disconnected"),
                Ok(result) => {
                    self.services.system.selection_worker = None;
                    result.map_err(anyhow::Error::msg)?
                }
            }
        } else {
            let storage = self
                .services
                .storage
                .clone()
                .context("Circus selection storage")?;
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("circus-selection-slots".into())
                .spawn(move || {
                    let result = (|| -> Result<Vec<Option<NativeSaveBody>>> {
                        let mut bodies = Vec::with_capacity(count);
                        for n in 0..count {
                            let slot = (count as i32)
                                .wrapping_mul(page)
                                .wrapping_add(n as i32)
                                .wrapping_add(3);
                            let path = VfsPath::new(Mount::Save, &SlotKind::Normal.filename(slot))?;
                            let body = if storage
                                .metadata(Mount::Save, path.relative())?
                                .is_some_and(|metadata| !metadata.is_directory)
                            {
                                let reader = storage.open_read(&path)?;
                                if reader.len()? < BODY_BYTES as u64 {
                                    None
                                } else {
                                    let mut bytes = vec![0; BODY_BYTES];
                                    crysirror_resource::read_exact_at(
                                        reader.as_ref(),
                                        0,
                                        &mut bytes,
                                    )?;
                                    Some(NativeSaveBody::from_exact(&bytes)?)
                                }
                            } else {
                                None
                            };
                            bodies.push(body);
                        }
                        Ok(bodies)
                    })();
                    let _ = sender.send(result.map_err(|error| format!("{error:#}")));
                })?;
            self.services.system.selection_worker = Some(SelectionWorker {
                mode,
                page,
                count,
                receiver,
            });
            return Ok(None);
        };
        let mut result = declarations.clone();
        result.rows.clear();
        result.current_row = None;
        for (n, body) in bodies.into_iter().enumerate() {
            let slot = (count as i32)
                .wrapping_mul(page)
                .wrapping_add(n as i32)
                .wrapping_add(3);
            let mut row = PanelRow {
                slot_id: slot,
                action: if mode == 3 || body.is_some() {
                    "selsave".into()
                } else {
                    String::new()
                },
                ..Default::default()
            };
            row.declared_image = true;
            row.states
                .push(part(19, [0, 0], metadata.save_crops[0], 99)?);
            let missing = body.is_none();
            for state in [1, 2] {
                row.states.push(part(
                    19,
                    [0, 0],
                    metadata.save_crops[if mode == 4 && missing {
                        0
                    } else {
                        state as usize
                    }],
                    state,
                )?);
            }
            for state in [99, 1, 2] {
                row.states.push(part(
                    19,
                    [14, 14],
                    if state == 99 || mode == 4 && missing {
                        [16, 127, 145, 202]
                    } else {
                        [148, 127, 277, 202]
                    },
                    state,
                )?);
            }
            let (date, title) = if let Some(body) = body {
                let [month, day, hour, minute, year, second] = body.civil_time_bytes();
                let date = format!(
                    "@i{} {:02}/{:02} {:02}:{:02}:{:02}",
                    u16::from(year) + 2000,
                    month,
                    day,
                    hour,
                    minute,
                    second
                );
                let scene = body.scene_name()?;
                let mapped = metadata
                    .save_titles
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(&scene))
                    .map(|(_, value)| value.as_str());
                let (title, prefix) = if let Some(mapped) = mapped {
                    let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(mapped);
                    ensure!(!error, "Circus selection title encoding");
                    if encoded.len() <= 1 {
                        (scene.as_str(), "@i")
                    } else {
                        (
                            mapped,
                            if encoded.len() + 2 > 34 {
                                "@r"
                            } else if encoded.len() + 2 > 30 {
                                "@y"
                            } else {
                                "@i"
                            },
                        )
                    }
                } else {
                    (scene.as_str(), "@i")
                };
                (date, format!("{prefix}{title}"))
            } else {
                ("@s---- --/-- --:--:--".into(), "@sNo Data".into())
            };
            //40B670's435A90 is a narrow wrapper, not a UTF16 setter:
            //435920(80,y,0,135,charText,0), followed by ++1924. Thus each
            //call advances the textbox but uses relative coordinates and
            //normal state99. 436050's separate wide-cache branch is not the
            //profile used by these built-in date/title declarations.
            for (y, text) in [(6, date), (25, title)] {
                row.texts.push(PanelText {
                    position: [80, y],
                    absolute: false,
                    variants: [(99, (135, text))].into(),
                });
            }
            if !missing && body_present_thumbnail(self, n, count, page) {
                let image = 230 + n as u8;
                for state in [99, 1, 2] {
                    row.states
                        .push(part(image, [16, 16], [0, 0, 125, 71], state)?);
                }
            }
            result.rows.push(row);
        }
        self.services.system.selection_commit = Some(SelectionCommit {
            mode,
            page,
            declarations: declarations.clone(),
            result: result.clone(),
        });
        if !self.refresh_panel_template(&result)? {
            return Ok(None);
        }
        self.services.system.selection_commit = None;
        Ok(Some(SystemReply::Selection(result)))
    }
}
fn body_present_thumbnail(bridge: &Bridge<'_>, n: usize, count: usize, page: i32) -> bool {
    n < 10
        && n as i32 + (count as i32).wrapping_mul(page) < 110
        && bridge.services.images.contains_key(&(230 + n as u8))
}
