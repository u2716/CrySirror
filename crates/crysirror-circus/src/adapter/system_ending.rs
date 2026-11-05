//! Native4595F0/459120 ending policy. Archive/image decoding and retained
//! position/opacity are reusable shared services; no legacy renderer is hosted.
use super::*;
const ROOT: u32 = 0x38d0_0000;
const FIRST: u32 = ROOT + 1;
const SECOND: u32 = ROOT + 2;
const FADE: u32 = ROOT + 3;
type Pages = std::result::Result<(usize, Vec<(usize, ImageSurface)>), String>;
#[derive(Clone, Copy)]
enum Phase {
    Scroll,
    Hold(u64),
    Fade { opacity: f32, last: u64 },
    Done(u64),
}
pub(super) struct Ending {
    parameter: i16,
    duration: u64,
    count: usize,
    page: i32,
    front: usize,
    back: Option<usize>,
    pages: BTreeMap<usize, UploadedImage>,
    worker: Option<Receiver<Pages>>,
    started: Option<u64>,
    phase: Phase,
    skip_pending: bool,
    advance_pending: bool,
}
impl Ending {
    pub(super) fn ui_waiting(&self) -> bool {
        self.started.is_some() && self.worker.is_none() && !self.skip_pending
    }
}
impl Bridge<'_> {
    pub(super) fn native_ending(
        &mut self,
        parameter: i16,
        duration: u64,
        skip_unlocked: bool,
    ) -> Result<Option<SystemReply>> {
        //4595F0 initializes only records8/9. Mode1 retains record0's
        // zero timing/page count, so its dormant crossfade branch cannot
        // advance the initialized scroll page. Preserve that native first
        // page until an unlocked skip, rather than inventing a slideshow.
        ensure!(
            duration <= u32::MAX as u64,
            "Circus native ending duration DWORD extent"
        );
        let mut state = self
            .services
            .system
            .ending
            .take()
            .unwrap_or_else(|| Ending {
                parameter,
                duration,
                count: 0,
                page: 0,
                front: 0,
                back: Some(1),
                pages: BTreeMap::new(),
                worker: None,
                started: None,
                phase: Phase::Scroll,
                skip_pending: false,
                advance_pending: false,
            });
        ensure!(
            state.parameter == parameter && state.duration == duration,
            "Circus ending completion ownership"
        );
        let result = self.tick_native_ending(&mut state, skip_unlocked);
        if !matches!(result, Ok(Some(_))) {
            self.services.system.ending = Some(state);
        }
        result
    }
    fn ending_pages(&mut self, state: &mut Ending, indices: &[usize]) -> Result<bool> {
        if let Some(worker) = &state.worker {
            match worker.try_recv() {
                Err(TryRecvError::Empty) => return Ok(false),
                Err(TryRecvError::Disconnected) => bail!("Circus ending page worker disconnected"),
                Ok(result) => {
                    state.worker = None;
                    let (count, pages) = result.map_err(anyhow::Error::msg)?;
                    state.count = count;
                    for (index, image) in pages {
                        let path = self.host.upload_rgba(
                            self.client,
                            0x3501_0000 + u32::try_from(index)?,
                            image.width,
                            image.height,
                            &image.rgba,
                        )?;
                        state.pages.insert(
                            index,
                            UploadedImage {
                                path,
                                size: [image.width, image.height],
                                opaque: image.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255),
                            },
                        );
                    }
                }
            }
        }
        let missing = indices
            .iter()
            .copied()
            .filter(|index| {
                !state.pages.contains_key(index) && (state.count == 0 || *index < state.count)
            })
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return Ok(true);
        }
        let storage = self
            .services
            .storage
            .as_ref()
            .context("Circus ending storage capability")?
            .clone();
        let client = self.client;
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("circus-ending-page".into())
            .spawn(move || {
                let result = (|| -> Result<(usize, Vec<(usize, ImageSurface)>)> {
                    let reader =
                        storage.open_read(&VfsPath::new(Mount::Asset, "Advdata/GRP/ED/ed.PCK")?)?;
                    let context = DecodeContext::new(client);
                    let archive =
                        crysirror_resource::circus_pck::CircusPck.open(reader, &context)?;
                    let count = archive.entries().len();
                    ensure!((1..=10000).contains(&count), "Circus ending page count");
                    let read = |id: u32| -> Result<Vec<u8>> {
                        crysirror_resource::read_all(
                            archive.open(id, &context)?.as_ref(),
                            64 * 1024 * 1024,
                        )
                    };
                    let mut pages = Vec::new();
                    for index in missing {
                        if index >= count {
                            continue;
                        }
                        let entry = &archive.entries()[index];
                        let bytes = read(entry.id)?;
                        let image = crysirror_image::crxd::decode(&bytes, &mut |dependency| {
                            let id = match dependency {
                                crysirror_image::crxd::Dependency::ArchiveOffset(offset) => archive
                                    .entries()
                                    .iter()
                                    .find(|entry| {
                                        entry.metadata["offset"].as_u64()
                                            == Some(u64::from(*offset))
                                    })
                                    .map(|entry| entry.id),
                                crysirror_image::crxd::Dependency::Name(name) => {
                                    archive.resolve(name)
                                }
                            }
                            .context("Circus ending page image dependency")?;
                            read(id)
                        })?;
                        pages.push((index, image.surface));
                    }
                    Ok((count, pages))
                })();
                let _ = sender.send(result.map_err(|e| format!("{e:#}")));
            })?;
        state.worker = Some(receiver);
        Ok(false)
    }
    fn ending_picture(&self, id: u32, image: Option<&UploadedImage>, y: i32) -> Value {
        match image {
            Some(image) => {
                json!({"op":"put","layer":{"id":id,"parent":ROOT,"rect":[0,y,image.size[0],image.size[1]],"paint":{"kind":"image","asset":image.path.relative()},"order":[0,if id==FIRST{0}else{1}]}})
            }
            None => {
                json!({"op":"put","layer":{"id":id,"parent":ROOT,"rect":[0,y,1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":[0,1]}})
            }
        }
    }
    fn tick_native_ending(
        &mut self,
        state: &mut Ending,
        skip_unlocked: bool,
    ) -> Result<Option<SystemReply>> {
        let now = self.host.now_ms();
        if state.started.is_none() {
            if !self.ending_pages(state, &[0, 1])? {
                return Ok(None);
            }
            self.submit(vec![
                json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[1_990_000_000,0]}}),
                self.ending_picture(FIRST,state.pages.get(&0),0),self.ending_picture(SECOND,state.pages.get(&1),720),
                json!({"op":"put","layer":{"id":FADE,"parent":ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"opacity":0,"order":[1,0]}}),
            ],vec![json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1280,720],"modal":true,"interaction_only":true,"key_events":[{"buttons":crysirror_core::service::buttons::MOUSE_RIGHT|crysirror_core::service::buttons::ESCAPE,"edge":"release","event":"ending_skip"},{"buttons":crysirror_core::service::buttons::MOUSE_LEFT|crysirror_core::service::buttons::ENTER|crysirror_core::service::buttons::PAGE_DOWN,"edge":"release","event":"ending_advance"}]}})])?;
            state.started = Some(now);
            return Ok(None);
        }
        // UI owns modal key edges; querying the retained receipt must not
        // re-filter against the already consumed native InputFrame.
        let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::UiSubtreeEvents(ROOT))?,
        )?;
        state.skip_pending |= events.iter().any(|event| event.kind == "ending_skip");
        state.advance_pending |= events.iter().any(|event| event.kind == "ending_advance");
        if state.parameter == -1 {
            self.submit(
                vec![json!({"op":"remove","id":ROOT})],
                vec![json!({"op":"remove","id":ROOT})],
            )?;
            for image in state.pages.values() {
                self.host
                    .release_image_resource(self.client, image.path.relative())?;
            }
            return Ok(Some(SystemReply::Done));
        }
        match state.phase {
            Phase::Scroll => {
                if state.skip_pending && skip_unlocked {
                    // Native43CC90(0)+43C1C0(0,180), already extracted into
                    // the shared release-tail service by the audio adapter.
                    if !self.release_music(false, 180)? {
                        return Ok(None);
                    }
                    state.phase = Phase::Fade {
                        opacity: 0.,
                        last: now,
                    };
                    state.skip_pending = false;
                    state.advance_pending = false;
                    return Ok(None);
                }
                state.skip_pending = false;
                state.advance_pending = false;
                if state.parameter == 1 {
                    return Ok(None);
                }
                //459120 uses signed FILD after DWORD subtraction. The native
                //SSE2 conversion helper45B550 returns INT_MIN for nonfinite or
                //out-of-range values; Rust's saturating float cast differs.
                let elapsed = (now as u32).wrapping_sub(state.started.unwrap() as u32) as i32;
                let duration = state.duration as u32 as i32;
                let rate = (720.0 * state.count as f64 / f64::from(duration)) as f32;
                let value = f64::from(elapsed) * f64::from(rate);
                let progress = if value.is_finite()
                    && value.trunc() >= i32::MIN as f64
                    && value.trunc() <= i32::MAX as f64
                {
                    value.trunc() as i32
                } else {
                    i32::MIN
                };
                let page = progress / 720;
                let mut first_y = progress.wrapping_neg() % 720;
                let mut second_y = 720 - progress % 720;
                if page != state.page {
                    let valid = usize::try_from(page)
                        .ok()
                        .filter(|index| *index < state.count);
                    if let Some(index) = valid {
                        if !self.ending_pages(state, &[index, index.saturating_add(1)])? {
                            return Ok(None);
                        }
                    }
                    state.page = page;
                    if valid.is_none() || valid == Some(state.count - 1) {
                        //4594DC/4594F1 branch does not attach the just-loaded
                        //last/invalid page. Its old renderer188 remains bound;
                        //only the two positions are reset before the hold.
                        first_y = 0;
                        second_y = 720;
                        state.phase = Phase::Hold(now);
                    } else {
                        state.front = valid.unwrap();
                        state.back = Some(state.front + 1);
                        first_y = first_y.wrapping_add(720);
                        second_y = second_y.wrapping_add(720);
                    }
                }
                self.submit(
                    vec![
                        self.ending_picture(FIRST, state.pages.get(&state.front), first_y),
                        self.ending_picture(
                            SECOND,
                            state.back.and_then(|index| state.pages.get(&index)),
                            second_y,
                        ),
                    ],
                    vec![],
                )?;
                let retire = state
                    .pages
                    .keys()
                    .copied()
                    .filter(|index| {
                        *index != state.front
                            && Some(*index) != state.back
                            && Some(*index) != usize::try_from(state.page).ok()
                    })
                    .collect::<Vec<_>>();
                for index in retire {
                    if let Some(image) = state.pages.remove(&index) {
                        self.host
                            .release_image_resource(self.client, image.path.relative())?;
                    }
                }
            }
            Phase::Hold(since) => {
                if state.skip_pending || state.advance_pending || now.saturating_sub(since) > 5000 {
                    state.phase = Phase::Fade {
                        opacity: 0.,
                        last: now,
                    };
                }
            }
            Phase::Fade { opacity, last } => {
                if now == last {
                    return Ok(None);
                }
                let opacity = (f64::from(opacity) + 0.004999999888241291) as f32;
                self.submit(
                    vec![json!({"op":"set_opacity","id":FADE,"opacity":(opacity.min(1.)*255.) as u8})],
                    vec![],
                )?;
                state.phase = if opacity >= 1. {
                    Phase::Done(now)
                } else {
                    Phase::Fade { opacity, last: now }
                };
            }
            Phase::Done(ended) => {
                if now == ended {
                    return Ok(None);
                }
                self.submit(
                    vec![json!({"op":"remove","id":ROOT})],
                    vec![json!({"op":"remove","id":ROOT})],
                )?;
                for image in state.pages.values() {
                    self.host
                        .release_image_resource(self.client, image.path.relative())?;
                }
                return Ok(Some(SystemReply::EndingComplete(if state.parameter == 1 {
                    110
                } else {
                    111
                })));
            }
        }
        Ok(None)
    }
}
