// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, HostServices, Result, Value, json};
use anyhow::Context;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::AssetReader,
};
use crysirror_image::{ImageSurface, sqz::Sequence};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, mpsc::Receiver},
};

#[derive(Default)]
pub(super) struct Published {
    pub ids: BTreeSet<u32>,
    pub tile_frames: Option<TileFrames>,
    sequences: BTreeMap<u32, Live>,
    tinted: BTreeMap<u32, Tinted>,
}
impl std::ops::Deref for Published {
    type Target = BTreeSet<u32>;
    fn deref(&self) -> &Self::Target {
        &self.ids
    }
}

pub(super) struct TileFrames {
    pub current: Option<ImageSurface>,
    pub history: ImageSurface,
}

struct Tinted {
    source: String,
    job: Option<Receiver<Result<ImageSurface>>>,
    path: Option<String>,
    op: Value,
}
struct Live {
    source: String,
    reader: Arc<dyn AssetReader>,
    format: Sequence,
    start: u64,
    manual: bool,
    desired: u32,
    displayed: Option<u32>,
    job: Option<(u32, Receiver<Result<ImageSurface>>)>,
    op: Value,
    dynamic: Option<String>,
}
impl Live {
    fn begin(&mut self) -> Result<()> {
        if self.job.is_some() || self.displayed == Some(self.desired) {
            return Ok(());
        }
        let (format, reader, frame) = (self.format, self.reader.clone(), self.desired);
        self.job = Some((
            frame,
            crate::worker(move || format.frame(reader.as_ref(), frame))?,
        ));
        Ok(())
    }
}
impl Published {
    pub fn finish_tiles(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        if let Some(frames) = &self.tile_frames && let Some(image) = &frames.current {
            h.present_surface(c, Self::slot(1)?, image.width, image.height, &image.rgba)?;
        }
        self.tile_frames = None;
        Ok(())
    }
    pub fn publish_tiles(&self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        if let Some(frames) = &self.tile_frames {
            let image = &frames.history;
            h.present_surface(c, Self::slot(1)?, image.width, image.height, &image.rgba)?;
        }
        Ok(())
    }
    pub fn ordinal(&self, id: u32) -> Option<u32> {
        self.sequences.get(&id)?.displayed
    }
    pub fn active(&self) -> bool {
        self.sequences
            .values()
            .any(|s| s.format.rate >= 0 || s.job.is_some())
    }
    pub fn pending(&self) -> bool {
        self.tinted.values().any(|t| t.job.is_some()) || self.sequences
            .values()
            .any(|s| s.job.is_some() && (s.displayed.is_none() || s.manual))
    }
    pub fn set_frame(&mut self, name: &str, ordinal: i32) -> Result<()> {
        let Some(s) = self
            .sequences
            .values_mut()
            .find(|s| s.source.split_once('/').is_some_and(|(_, n)| n == name))
        else {
            return Ok(());
        };
        if s.format.rate < 0 && ordinal != i32::MAX {
            // A negative signed C remainder does not identify a decodable frame.
            let frame = i64::from(ordinal) % i64::from(s.format.count);
            if frame >= 0 {
                s.desired = frame as u32;
                s.manual = true;
                s.begin()?;
            }
        }
        Ok(())
    }
    // Resolve name-bound animated bitmaps before a layer transaction. Keep geometry
    // changes on a live frame and retire jobs only after the scene accepts its batch.
    pub fn send(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        layers: &mut [Value],
    ) -> Result<()> {
        enum Binding {
            Remove,
            Keep(Value),
            Replace(Live),
        }
        let mut updates = BTreeMap::new();
        for op in layers.iter_mut() {
            let (id, path) = match op["op"].as_str() {
                Some("remove") => (op["id"].as_u64().unwrap() as u32, None),
                Some("put") => (
                    op["layer"]["id"].as_u64().unwrap() as u32,
                    op["layer"]["paint"]["asset"].as_str().map(str::to_owned),
                ),
                _ => continue,
            };
            if let Some((tone, raw)) = path.as_deref().and_then(super::tone::path) {
                let source = path.as_ref().unwrap();
                if self.tinted.get(&id).is_none_or(|t| &t.source != source) {
                    let job = super::tone::prepare(h,c,tone,raw)?;
                    let prev = self.tinted.remove(&id);
                    if prev.is_some_and(|t| t.path.is_some()) { h.release_surface(c, Self::tint_slot(id)?)?; }
                    self.tinted.insert(id,Tinted { source:source.clone(),job:Some(job),path:None,op:op.clone() });
                }
                let tint = self.tinted.get_mut(&id).unwrap();
                tint.op = op.clone();
                if let Some(real) = &tint.path {
                    op["layer"]["paint"]["asset"] = json!(real);
                } else { op["layer"]["paint"] = json!({"kind":"solid","color":[0,0,0,0]}); }
            } else if self.tinted.remove(&id).is_some_and(|t| t.path.is_some()) {
                h.release_surface(c, Self::tint_slot(id)?)?;
            }
            let Some(path) = path.filter(|p| p.to_ascii_lowercase().ends_with(".sqz")) else {
                updates.insert(id, Binding::Remove);
                continue;
            };
            let current = match updates.get(&id) {
                Some(Binding::Replace(record)) => Some(record),
                Some(Binding::Remove) => None,
                _ => self.sequences.get(&id),
            };
            let layer = op.clone();
            if let Some(record) = current.filter(|s| s.source == path) {
                if let Some(dynamic) = &record.dynamic {
                    op["layer"]["paint"]["asset"] = json!(dynamic);
                } else {
                    op["layer"]["paint"] = json!({"kind":"solid","color":[0,0,0,0]});
                }
                match updates.get_mut(&id) {
                    Some(Binding::Replace(record)) => record.op = layer,
                    _ => {
                        updates.insert(id, Binding::Keep(layer));
                    }
                }
            } else {
                let reader = h
                    .storage(c)?
                    .open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
                let format = Sequence::open(reader.as_ref())?;
                let mut record = Live {
                    source: path,
                    reader,
                    format,
                    start: h.now_ms(),
                    manual: false,
                    desired: 1 % format.count,
                    displayed: None,
                    job: None,
                    op: layer,
                    dynamic: None,
                };
                record.begin()?;
                updates.insert(id, Binding::Replace(record));
                op["layer"]["paint"] = json!({"kind":"solid","color":[0,0,0,0]});
            }
        }
        h.submit_engine(c, 1, &serde_json::to_vec(&json!({"layers":layers}))?)?;
        for (id, update) in updates {
            match update {
                Binding::Keep(op) => {
                    if let Some(record) = self.sequences.get_mut(&id) {
                        record.op = op;
                    }
                }
                Binding::Remove => self.release(h, c, id)?,
                Binding::Replace(record) => {
                    self.release(h, c, id)?;
                    self.sequences.insert(id, record);
                }
            }
        }
        Ok(())
    }
    fn tint_slot(id: u32) -> Result<u32> { id.checked_add(0x3000_0000).context("minori tinted surface ID") }
    fn slot(id: u32) -> Result<u32> {
        id.checked_add(0x2000_0000).context("SQZ surface ID")
    }
    fn release(&mut self, h: &mut dyn HostServices, c: ClientId, id: u32) -> Result<()> {
        if self
            .sequences
            .remove(&id)
            .is_some_and(|s| s.dynamic.is_some())
        {
            h.release_surface(c, Self::slot(id)?)?;
        }
        Ok(())
    }
    pub fn tick(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        for (&id, tint) in &mut self.tinted {
            if let Some(job) = &tint.job && let Some(image) = crate::poll(job)? {
                let path = h.present_surface(c,Self::tint_slot(id)?,image.width,image.height,&image.rgba)?.relative().to_owned();
                let mut op = tint.op.clone();
                op["layer"]["paint"]["asset"] = json!(path);
                h.submit_engine(c,1,&serde_json::to_vec(&json!({"layers":[op]}))?)?;
                tint.path = Some(path);
                tint.job = None;
            }
        }
        for (&id, s) in &mut self.sequences {
            if let Some((frame, job)) = &s.job {
                if let Some(img) = crate::poll(job)? {
                    let image = if id == 1 && let Some(frames) = &mut self.tile_frames {
                        frames.current = Some(img);
                        &frames.history
                    } else {
                        &img
                    };
                    let path = h
                        .present_surface(c, Self::slot(id)?, image.width, image.height, &image.rgba)?
                        .relative()
                        .to_owned();
                    if s.dynamic.as_ref() != Some(&path) {
                        let mut op = s.op.clone();
                        op["layer"]["paint"]["asset"] = json!(path);
                        h.submit_engine(c, 1, &serde_json::to_vec(&json!({"layers":[op]}))?)?;
                        s.dynamic = Some(path);
                    }
                    s.displayed = Some(*frame);
                    if *frame == s.desired {
                        s.manual = false;
                    }
                    s.job = None;
                }
            }
            if s.displayed.is_some() && s.format.rate >= 0 {
                let elapsed = h.now_ms().saturating_sub(s.start);
                let advancement = if s.format.rate == 0 {
                    u128::from(s.displayed.unwrap())
                } else {
                    u128::from(elapsed) * s.format.rate as u128 / 1000
                };
                s.desired = ((1 + advancement) % u128::from(s.format.count)) as u32;
            }
            s.begin()?;
        }
        Ok(())
    }
}
