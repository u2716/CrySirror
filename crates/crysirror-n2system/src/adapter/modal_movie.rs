// SPDX-License-Identifier: MPL-2.0
//! Trial4147E0/472960: standalone DirectShow movie, blocking the whole VM.
//! The native audio-enable state is restored after completion. WndProc475450
//! admits left-button skip when requested; Enter opens skip confirmation.
//! Scoped shared media owns decoding, transport and output on every host.
use super::*;
use crysirror_core::service::{MediaPhase, MediaRequest};

pub(super) struct ModalMovie {
    owner: u32,
    source: String,
    allow_skip: bool,
    gain: f32,
    tickets: Vec<u64>,
    channels: BTreeSet<u32>,
    paused_audio: Vec<u32>,
    paused_media: Vec<u32>,
    hidden_widgets: Vec<u32>,
    root: Option<u32>,
    video: Option<u32>,
    surface: Option<String>,
    initialized: bool,
    shown: bool,
    confirming: bool,
}
impl N2SystemVm {
    pub(super) fn native_modal_movie(
        &mut self,
        _host: &mut dyn HostServices,
        owner: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(args.len() == 2, "N2System MoviePlay signature");
        if self.modal_movie_results.remove(&owner) {
            self.machine.complete(&mut state.thread, Value::Null)?;
            return Ok(true);
        }
        ensure!(self.modal_movie.is_none(), "N2System nested modal movie");
        let source = resource::resolve(self.catalog.as_ref().unwrap(), &args.string(0)?)?;
        // Native string parser accepts TRUE case-insensitively, not arbitrary
        // nonempty strings. Do not treat a nonzero integer as this flag.
        let allow_skip = args.string(1)?.eq_ignore_ascii_case("TRUE");
        let gain = self
            .machine
            .flags
            .get("SYSTEM_sound_volume_movie")
            .map(Value::number)
            .transpose()?
            .unwrap_or(1000.)
            / 1000.;
        ensure!(gain.is_finite(), "N2System MoviePlay volume");
        let mut tickets = self.voice_tickets.clone();
        tickets.extend_from_slice(&state.audio);
        for thread in self.threads.values() {
            tickets.extend_from_slice(&thread.audio);
        }
        tickets.sort_unstable();
        tickets.dedup();
        self.modal_movie = Some(ModalMovie {
            owner,
            source,
            allow_skip,
            gain: gain.clamp(0., 1.),
            tickets,
            channels: self
                .sound_resources
                .keys()
                .copied()
                .chain(self.voices.values().copied())
                .collect(),
            paused_audio: Vec::new(),
            paused_media: Vec::new(),
            hidden_widgets: Vec::new(),
            root: None,
            video: None,
            surface: None,
            initialized: false,
            shown: false,
            confirming: false,
        });
        Ok(false)
    }

    fn prepare_modal_movie(
        &mut self,
        host: &mut dyn HostServices,
        modal: &mut ModalMovie,
    ) -> Result<bool> {
        // Finish already-admitted audio publications before pausing. No script
        // runs during this fence, so a pending voice cannot start over a movie.
        let mut pending = false;
        for &ticket in &modal.tickets {
            let receipt = host.audio_preparation(self.client, ticket)?;
            modal.channels.insert(receipt.channel);
            match receipt.phase {
                AudioPreparePhase::Pending => pending = true,
                AudioPreparePhase::Ready => {}
                AudioPreparePhase::Failed => bail!(
                    "N2System pre-movie audio: {}",
                    receipt.error.unwrap_or_default()
                ),
            }
        }
        if pending {
            return Ok(false);
        }
        for &channel in &modal.channels {
            if host
                .audio_info(self.client, channel)?
                .is_some_and(|info| !info.paused)
            {
                host.submit_audio(
                    self.client,
                    AudioRequest::Pause {
                        channel,
                        paused: true,
                        fade_ms: 0.,
                    },
                )?;
                modal.paused_audio.push(channel);
            }
        }
        for &id in self.movies.keys() {
            let info = host.media_info(self.client, id)?;
            if !info.paused && matches!(info.phase, MediaPhase::Loading | MediaPhase::Playing) {
                host.media_request(self.client, MediaRequest::Pause { id })?;
                modal.paused_media.push(id);
            }
        }
        let widgets: Vec<_> = self
            .choices
            .iter()
            .filter(|(_, choice)| choice.widget)
            .map(|(&id, _)| id)
            .chain(self.scrollbars.keys().copied())
            .collect();
        for id in widgets {
            let widget: Json = serde_json::from_slice(
                &host.query_engine(self.client, EngineQuery::UiWidget(id))?,
            )?;
            if widget["visible"] == true {
                self.submit(
                    host,
                    vec![],
                    vec![json!({"op":"set_visible","id":id,"visible":false})],
                )?;
                modal.hidden_widgets.push(id);
            }
        }
        // These are private session-owned identities, never script destinations
        // or aliases. A root prevents inherited camera/matte transforms.
        let size = self.config.canvas;
        let root = self.entities.add(
            "\0modal-movie".into(),
            None,
            Kind::Name,
            size.map(|v| v as f32),
        )?;
        modal.root = Some(root);
        let video = self.entities.add(
            "\0video".into(),
            Some(root),
            Kind::Visual,
            size.map(|v| v as f32),
        )?;
        modal.video = Some(video);
        self.submit(host, vec![
            json!({"op":"put","layer":{"id":root,"order":[i32::MAX,0],"rect":[0,0,size[0],size[1]],"paint":{"kind":"solid","color":[0,0,0,255]}}}),
        ], vec![])?;
        modal.surface = Some(
            host.media_request(
                self.client,
                MediaRequest::Play {
                    id: video,
                    asset: modal.source.clone(),
                    looping: false,
                    audio: true,
                    gain: modal.gain,
                },
            )?
            .relative()
            .to_owned(),
        );
        modal.initialized = true;
        Ok(true)
    }

    pub(super) fn advance_modal_movie(
        &mut self,
        host: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<()> {
        let Some(mut modal) = self.modal_movie.take() else {
            return Ok(());
        };
        let result = (|| -> Result<bool> {
            if !modal.initialized && !self.prepare_modal_movie(host, &mut modal)? {
                return Ok(false);
            }
            let video = modal.video.unwrap();
            if modal.confirming {
                let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(
                    &host.query_engine(self.client, EngineQuery::DialogEvents)?,
                )?;
                for event in events {
                    ensure!(
                        event.widget.owner == self.client.0
                            && event.widget.id == 65535
                            && event.kind == "dialog_result",
                        "N2System movie confirmation receipt"
                    );
                    modal.confirming = false;
                    if event.value == 1. {
                        return Ok(true);
                    }
                    ensure!(event.value == 2., "N2System movie confirmation value");
                    host.media_request(self.client, MediaRequest::Resume { id: video })?;
                }
                return Ok(false);
            }
            let info = host.media_info(self.client, video)?;
            ensure!(
                info.phase != MediaPhase::Failed,
                "N2System MoviePlay decoder failed: {}",
                modal.source
            );
            if info.size.is_some() && !modal.shown {
                let [w, h] = self.config.canvas;
                self.submit(host, vec![json!({"op":"put","layer":{"id":video,"parent":modal.root,"rect":[0,0,w,h],"paint":{"kind":"image","asset":modal.surface.as_ref().unwrap()}}})], vec![])?;
                modal.shown = true;
            }
            if modal.allow_skip && input.buttons_pressed & buttons::ENTER != 0 {
                use crysirror_scene::ui::dialog::{DialogButton, DialogSpec};
                host.media_request(self.client, MediaRequest::Pause { id: video })?;
                let dialog = DialogSpec {
                    id: 65535,
                    title: String::new(),
                    message: "Stop movie playback?".into(),
                    buttons: vec![
                        DialogButton {
                            id: 1,
                            label: "Yes".into(),
                        },
                        DialogButton {
                            id: 2,
                            label: "No".into(),
                        },
                    ],
                    default: 1,
                    cancel: Some(2),
                    font: self.style.font.clone(),
                    text_size: self.style.size.clamp(8, 128),
                };
                self.submit(
                    host,
                    vec![],
                    vec![json!({"op":"open_dialog","dialog":dialog})],
                )?;
                modal.confirming = true;
                return Ok(false);
            }
            Ok(
                matches!(info.phase, MediaPhase::Ended | MediaPhase::Stopped)
                    || (modal.allow_skip && input.buttons_pressed & buttons::MOUSE_LEFT != 0),
            )
        })();
        // Native input is owned by the modal window throughout playback and
        // on its last tick. It must not advance text or activate a choice.
        input.buttons_down = 0;
        input.buttons_pressed = 0;
        match result {
            Ok(false) => self.modal_movie = Some(modal),
            Ok(true) => {
                let owner = modal.owner;
                self.finish_modal_movie(host, modal)?;
                self.modal_movie_results.insert(owner);
            }
            Err(error) => {
                self.finish_modal_movie(host, modal)
                    .with_context(|| format!("N2System movie cleanup after {error:#}"))?;
                return Err(error);
            }
        }
        Ok(())
    }

    fn finish_modal_movie(&mut self, host: &mut dyn HostServices, modal: ModalMovie) -> Result<()> {
        // Attempt every restoration even if an output service failed. Preserve
        // the first failure instead of abandoning the remaining paused owners.
        let mut failure: Option<anyhow::Error> = None;
        let mut remember = |result: Result<()>| {
            if let Err(error) = result {
                if failure.is_none() {
                    failure = Some(error);
                }
            }
        };
        if modal.confirming {
            remember(self.submit(host, vec![], vec![json!({"op":"close_dialog","id":65535})]));
        }
        if let Some(id) = modal.video {
            remember(
                host.media_request(self.client, MediaRequest::Remove { id })
                    .map(|_| ())
                    .map_err(Into::into),
            );
        }
        if let Some(root) = modal.root {
            remember(self.destroy(host, root));
        }
        for channel in modal.paused_audio {
            remember(
                host.submit_audio(
                    self.client,
                    AudioRequest::Pause {
                        channel,
                        paused: false,
                        fade_ms: 0.,
                    },
                )
                .map_err(Into::into),
            );
        }
        for id in modal.paused_media {
            remember(
                host.media_request(self.client, MediaRequest::Resume { id })
                    .map(|_| ())
                    .map_err(Into::into),
            );
        }
        for id in modal.hidden_widgets {
            remember(self.submit(
                host,
                vec![],
                vec![json!({"op":"set_visible","id":id,"visible":true})],
            ));
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    pub(super) fn cancel_modal_movie(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(modal) = self.modal_movie.take() {
            self.finish_modal_movie(host, modal)?;
        }
        Ok(())
    }
}
