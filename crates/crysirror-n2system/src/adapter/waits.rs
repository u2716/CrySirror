// SPDX-License-Identifier: MPL-2.0
//! Captured wait targets and input consumption follow Thread.cs/AudioEntity.cs,
//! NitroSharp 77a93db4 (MIT). Observe completions, never infer them from a PC.
use super::*;
use crysirror_core::service::{MediaPhase, MediaRequest};

impl N2SystemVm {
    pub(super) fn playback_wait_active(
        &self,
        host: &mut dyn HostServices,
        id: u32,
    ) -> Result<bool> {
        if self.movies.contains_key(&id) {
            let info = host.media_info(self.client, id)?;
            ensure!(
                info.phase != MediaPhase::Failed,
                "N2System waited movie failed"
            );
            return Ok(
                !info.paused && !matches!(info.phase, MediaPhase::Ended | MediaPhase::Stopped)
            );
        }
        if matches!(
            self.entities.nodes.get(&id).map(|e| &e.kind),
            Some(Kind::Sound { .. })
        ) {
            return Ok(host.audio_info(self.client, id)?.is_some_and(|info| {
                info.gain_fade_frames > 0
                    || (!info.looping
                        && info.source_playing
                        && !info.paused
                        && info.transport_active
                        && info.gain != 0.)
            }));
        }
        Ok(false)
    }
    pub(super) fn action_wait_active(&self, host: &mut dyn HostServices, id: u32) -> Result<bool> {
        let Some(entity) = self.entities.nodes.get(&id) else {
            return Ok(false);
        };
        if self.legacy_transitions.contains_key(&id) { return Ok(true); }
        if let Some(camera) = self.cameras.get(&id) {
            return Ok(camera.active(host.now_ms()));
        }
        if self
            .cubes
            .get(&id)
            .is_some_and(|c| c.active(host.now_ms(), &[]))
        {
            return Ok(true);
        }
        if let Some(blur) = self.blurs.get(&id) {
            if blur.active(host.now_ms())? {
                return Ok(true);
            }
        }
        if self
            .ripples
            .get(&id)
            .is_some_and(|r| r.active(host.now_ms()))
        {
            return Ok(true);
        }
        if self.shakes.contains_key(&id) || self.shades.get(&id).is_some_and(shade::Shade::pending)
        {
            return Ok(true);
        }
        if matches!(entity.kind, Kind::Thread) {
            return Ok(self
                .threads
                .get(&id)
                .is_some_and(|s| !s.thread.halted && s.enabled));
        }
        if matches!(entity.kind, Kind::Sound { .. }) {
            if self.playback_wait_active(host, id)? {
                return Ok(true);
            }
            return Ok(self
                .sound_resources
                .get(&id)
                .and_then(|s| s.rate_tween.as_ref())
                .is_some_and(|t| !t.complete(host.now_ms())));
        }
        if !matches!(entity.kind, Kind::Visual) {
            return Ok(false);
        }
        let now = host.now_ms();
        if self
            .paths
            .get(&id)
            .is_some_and(|t| !t.progress.complete(now))
            || self
                .mask_transitions
                .get(&id)
                .is_some_and(|t| !t.progress.complete(now))
            || self
                .frame_animations
                .get(&id)
                .is_some_and(|t| !t.clock.complete)
            || self
                .text_fades
                .get(&id)
                .is_some_and(|t| !t.clock.complete())
            || self.text_jobs.contains_key(&id)
            || self.text_dirty.contains(&id)
        {
            return Ok(true);
        }
        let motion: Json =
            serde_json::from_slice(&host.query_engine(self.client, EngineQuery::LayerMotion(id))?)?;
        if motion["tracks"]
            .as_array()
            .context("N2System action motion receipt")?
            .iter()
            .any(|track| track["complete"] != true)
        {
            return Ok(true);
        }
        let layer: crysirror_scene::layer::LayerSpec =
            serde_json::from_value(self.layer(host, id)?)?;
        Ok(layer
            .paint
            .glyph_reveal_status()
            .is_some_and(|s| !s.complete))
    }
    pub(super) fn interrupt_movie_wait(
        &mut self,
        host: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<()> {
        let mask = buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT | buttons::ENTER | buttons::SPACE;
        if input.buttons_pressed & mask == 0 {
            return Ok(());
        }
        for state in self.threads.values() {
            if !state.enabled || state.thread.halted {
                continue;
            }
            let Wait::Audio {
                ids,
                allow_movie_skip,
            } = &state.wait
            else {
                continue;
            };
            let mut active = Vec::new();
            for &id in ids {
                if self.movies.contains_key(&id) && self.playback_wait_active(host, id)? {
                    active.push(id);
                }
            }
            if active.is_empty() {
                continue;
            }
            // A movie consumes its interrupt key even when its caller forbids
            // skipping. It must not also activate a dialogue/choice/menu.
            input.buttons_pressed &= !mask;
            if *allow_movie_skip {
                for id in active {
                    host.media_request(self.client, MediaRequest::Stop { id })?;
                }
            }
            break;
        }
        Ok(())
    }
}
