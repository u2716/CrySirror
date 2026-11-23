// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS movie objects (6FC410 family, natives 265..270, 417, 787, 788) over the shared
//! media service: the script only opens, places and polls; decoding stays in the engine.
use anyhow::{Context, Result};
use crysirror_core::{
    service::{HostServices, MediaPhase, MediaRequest},
    state::ClientId,
};
use std::collections::BTreeMap;

const FIRST_MEDIA: u32 = 0x4e00_0001;
/// Layers of playing movies: above everything the script draws.
const LAYER: u32 = 0x1e00_0000;

struct Movie {
    // VM handles retain their native namespace; shared media IDs are client-scoped
    // and must remain below 1024. Reclaim this ID when the native object closes.
    media_id: u32,
    asset: String,
    rect: [i32; 4],
    gain: f32,
    started: bool,
    /// 6FD120 set +0x174 and 6FD6D0 has not cleared it.
    running: bool,
}
#[derive(Default)]
pub(crate) struct Movies {
    next: u32,
    movies: BTreeMap<u32, Movie>,
}
impl Movies {
    /// 6FCC50: a movie named by the resource catalog; the VM keeps the id.
    pub fn open(&mut self, asset: String, canvas: [u32; 2]) -> Result<u32> {
        let media_id = (1..1024).find(|candidate| self.movies.values().all(|movie| movie.media_id != *candidate))
            .context("CatSystem2 movie media ID budget")?;
        let id = self.next.max(FIRST_MEDIA);
        self.next = id.checked_add(1).context("CatSystem2 movie handle range")?;
        self.movies.insert(id, Movie { media_id, asset, rect: [0, 0, canvas[0] as i32, canvas[1] as i32], gain: 1., started: false, running: false });
        Ok(id)
    }
    fn layer(id: u32) -> u32 {
        LAYER | (id & 0xffff)
    }
    /// 6FD120: start playback where 6FD590 placed it.
    pub fn play(&mut self, host: &mut dyn HostServices, client: ClientId, id: u32) -> Result<()> {
        let Some(movie) = self.movies.get_mut(&id) else { return Ok(()) };
        let texture = host.media_request(client, MediaRequest::Play { id: movie.media_id, asset: movie.asset.clone(), looping: false, audio: true, gain: movie.gain })?;
        movie.started = true;
        movie.running = true;
        let [x, y, w, h] = movie.rect;
        host.submit_engine(client, 1, &serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":Self::layer(id),"rect":[x,y,w,h],"order":[1_000_000,0],"paint":{"kind":"image","asset":texture.relative()}}}]}))?)?;
        Ok(())
    }
    /// 6FD4D0.
    pub fn set_gain(&mut self, host: &mut dyn HostServices, client: ClientId, id: u32, percent: u32) -> Result<()> {
        let Some(movie) = self.movies.get_mut(&id) else { return Ok(()) };
        movie.gain = (percent as f32 / 100.).clamp(0., 1.);
        if movie.started {
            host.media_request(client, MediaRequest::SetGain { id: movie.media_id, gain: movie.gain })?;
        }
        Ok(())
    }
    /// 6FD590.
    pub fn place(&mut self, host: &mut dyn HostServices, client: ClientId, id: u32, rect: [i32; 4]) -> Result<()> {
        let Some(movie) = self.movies.get_mut(&id) else { return Ok(()) };
        movie.rect = rect;
        if movie.started {
            let [x, y, w, h] = rect;
            host.submit_engine(client, 1, &serde_json::to_vec(&serde_json::json!({"layers":[{"op":"set_rect","id":Self::layer(id),"rect":[x,y,w,h]}]}))?)?;
        }
        Ok(())
    }
    /// 6FCC30: 1 until the picture has ended or could not be decoded.
    pub fn playing(&self, host: &dyn HostServices, client: ClientId, id: u32) -> u32 {
        let Some(movie) = self.movies.get(&id) else { return 0 };
        if !movie.started {
            return 0;
        }
        host.media_info(client, movie.media_id).ok().is_some_and(|info| matches!(info.phase, MediaPhase::Loading | MediaPhase::Playing)) as u32
    }
    /// 6FD6D0: a running movie's graph stops and its picture hides; answers
    /// whether it was running.
    pub fn stop(&mut self, host: &mut dyn HostServices, client: ClientId, id: u32) -> Result<bool> {
        let Some(movie) = self.movies.get_mut(&id).filter(|movie| movie.running) else { return Ok(false) };
        movie.running = false;
        host.media_request(client, MediaRequest::Stop { id: movie.media_id })?;
        host.submit_engine(client, 1, &serde_json::to_vec(&serde_json::json!({"layers":[{"op":"set_visible","id":Self::layer(id),"visible":false}]}))?)?;
        Ok(true)
    }
    /// 654C10 → 6FCA60: IMediaSeeking current position (100 ns) / 10 / 1000.
    pub fn position_ms(&self, host: &dyn HostServices, client: ClientId, id: u32) -> u32 {
        let Some(movie) = self.movies.get(&id).filter(|movie| movie.started) else { return 0 };
        host.media_info(client, movie.media_id).map_or(0, |info| info.position_ms as u32)
    }
    /// 6FD6D0 + 6FC4E0.
    pub fn close(&mut self, host: &mut dyn HostServices, client: ClientId, id: u32) -> Result<()> {
        let Some(movie) = self.movies.remove(&id) else { return Ok(()) };
        if movie.started {
            let _ = host.media_request(client, MediaRequest::Remove { id: movie.media_id });
            host.submit_engine(client, 1, &serde_json::to_vec(&serde_json::json!({"layers":[{"op":"remove","id":Self::layer(id)}]}))?)?;
        }
        Ok(())
    }
}
