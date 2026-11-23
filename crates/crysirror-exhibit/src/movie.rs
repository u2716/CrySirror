// SPDX-License-Identifier: MPL-2.0
//! RLD movie channel and its independent optional channel-1 effect.
use anyhow::{bail, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AudioAsset, AudioParameters, AudioRequest, HostServices, MediaPhase, MediaRequest},
    state::ClientId,
};
use serde_json::json;

const CHANNEL: u32 = 1;
const LAYER: u32 = 2;
const EFFECT_CHANNEL: u32 = 15;

#[derive(Debug, PartialEq)]
pub(super) struct Movie {
    pub path: String,
    pub sound: i32,
    pub rect: [i32; 4],
}
pub(super) struct Playing {
    opened: u64,
    has_sound: bool,
    removed: bool,
}
impl Playing {
    pub fn start(
        client: ClientId,
        h: &mut dyn HostServices,
        canvas: [u32; 2],
        movie: Movie,
    ) -> Result<Self> {
        let [x, y, w, height] = movie.rect;
        let w = if w == 0 { canvas[0] as i32 } else { w };
        let height = if height == 0 {
            canvas[1] as i32
        } else {
            height
        };
        let at = h.media_request(
            client,
            MediaRequest::Play {
                id: CHANNEL,
                asset: movie.path,
                looping: false,
                audio: true,
                gain: 1.0,
            },
        )?;
        let result = (|| -> Result<()> {
            if movie.sound >= 0 {
                let path = VfsPath::new(Mount::Asset, &format!("res/s/e/{:03}.wav", movie.sound))?;
                h.submit_audio(
                    client,
                    AudioRequest::PlayAsset {
                        channel: EFFECT_CHANNEL,
                        source: AudioAsset {
                            path,
                            frame_limit: None,
                        },
                        intro: None,
                        parameters: AudioParameters::default(),
                    },
                )?;
            }
            let layer = json!({"layers":[{"op":"put","layer":{"id":LAYER,"order":[1,0],"rect":[x,y,w,height],"paint":{"kind":"image","asset":at.relative()}}}]});
            h.submit_engine(client, 1, &serde_json::to_vec(&layer)?)?;
            Ok(())
        })();
        if let Err(error) = result {
            let _ = h.media_request(client, MediaRequest::Remove { id: CHANNEL });
            if movie.sound >= 0 {
                let _ = h.submit_audio(
                    client,
                    AudioRequest::Stop {
                        channel: EFFECT_CHANNEL,
                        fade_ms: 0.0,
                    },
                );
            }
            return Err(error);
        }
        Ok(Self {
            opened: h.now_ms(),
            has_sound: movie.sound >= 0,
            removed: false,
        })
    }
    /// leaveMovie enforces a one-second startup interval before even a
    /// nonblocking leave. With sound duration zero it then waits for the WAV.
    pub fn leave(
        &mut self,
        client: ClientId,
        h: &mut dyn HostServices,
        wait: bool,
        fade: i32,
    ) -> Result<Option<u64>> {
        let now = h.now_ms();
        if now < self.opened.saturating_add(1000) {
            return Ok(Some(self.opened.saturating_add(1000)));
        }
        if !self.removed {
            match h.media_info(client, CHANNEL)?.phase {
                MediaPhase::Failed => bail!("ExHibit movie decoder failed"),
                MediaPhase::Loading | MediaPhase::Playing if wait => {
                    return Ok(Some(now.saturating_add(1)));
                }
                _ => {}
            }
            h.submit_engine(
                client,
                1,
                &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":LAYER}]}))?,
            )?;
            h.media_request(client, MediaRequest::Remove { id: CHANNEL })?;
            self.removed = true;
        }
        if self.has_sound {
            if fade == 0 && h.audio_playing(client, EFFECT_CHANNEL)? {
                return Ok(Some(now.saturating_add(1)));
            }
            h.submit_audio(
                client,
                AudioRequest::Stop {
                    channel: EFFECT_CHANNEL,
                    fade_ms: fade.max(0) as f32,
                },
            )?;
        }
        Ok(None)
    }
}
