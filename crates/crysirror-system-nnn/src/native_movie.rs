// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original outer printmovie selection/wait policy over the shared Media service.
use crate::{
    expression::Variables,
    native_bootstrap::NativeConfiguration,
    native_services_bootstrap::NativePreferences,
    native_shell::{NativeMode, NativeStartupProfile},
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{HostServices, InputFrame, MediaPhase, MediaRequest},
    state::ClientId,
};

#[derive(Clone, Debug)]
pub struct NativeMovieSpec {
    pub asset: Option<String>,
    pub next: NativeMode,
    pub no_skip: bool,
    pub last_wait: u32,
    pub exit_fill: Option<[u8; 3]>,
    pub gain: f32,
}
impl NativeMovieSpec {
    /// `None` means the actual selectByVar=-1, existing-variable-zero branch.
    /// Missing variables/lists retain the original default movie fallback.
    pub fn from_profile(
        profile: &NativeStartupProfile,
        configuration: &NativeConfiguration,
        variables: &Variables,
        preferences: &NativePreferences,
    ) -> Result<Self> {
        let table = &profile.movie;
        let int =
            |key: &[u8], default| -> Result<i32> { Ok(table.integer(key)?.unwrap_or(default)) };
        let selection = int(b"selectByVarFlag", 0)?;
        let default_name = table.parameter(b"movieFileName").unwrap_or(b"default.wmv");
        let encoded_default = encoding_rs::SHIFT_JIS.encode("ムービー番号").0;
        let name = table
            .parameter(b"selectMovieVarName")
            .unwrap_or(encoded_default.as_ref());
        let variable = if selection != 0 {
            configuration.variable_names.index(name)
        } else {
            None
        };
        let value = variable
            .map(|id| variables.read_calculated(id))
            .transpose()?;
        let chosen = if selection == -1 && value == Some(0) {
            None
        } else {
            let candidate = value
                .filter(|n| *n > 0)
                .and_then(|n| {
                    profile
                        .movie_names
                        .as_ref()
                        .and_then(|list| list.tokens.get((n as usize - 1) * 2))
                        .map(Vec::as_slice)
                })
                .unwrap_or(default_name);
            Some(
                configuration
                    .resources
                    .encoded_path(&[b"movie\\".as_slice(), candidate].concat())?,
            )
        };
        let last_wait = u32::try_from(int(b"lastWait", 5)?)?;
        ensure!(last_wait <= 864000, "NNN movie lastWait bounds");
        // The reviewed legacy constructor has no fixed-size hook. Do not
        // import later source behavior into this module profile.
        let fixed = table
            .integer(b"fixedMovieSizeX")?
            .zip(table.integer(b"fixedMovieSizeY")?);
        ensure!(
            fixed.is_none(),
            "NNN non-default fixed movie size needs native placement policy"
        );
        ensure!(
            int(b"mustOpeningMovieFlagOn", 0)? <= 0,
            "NNN movie acquisition flags require original SystemFile persistence"
        );
        let next = table
            .parameter(b"nextMode")
            .map(NativeMode::named)
            .transpose()?
            .unwrap_or(NativeMode::Title);
        let exit_fill = if int(b"fillExitScreenFlag", 1)? != 0 {
            Some([
                int(b"exitFadeOutColorR", 0)? as u8,
                int(b"exitFadeOutColorG", 0)? as u8,
                int(b"exitFadeOutColorB", 0)? as u8,
            ])
        } else {
            None
        };
        Ok(Self {
            asset: chosen,
            next,
            no_skip: int(b"noSKipFlag", 0)? != 0,
            last_wait,
            exit_fill,
            gain: crate::native_audio::legacy_gain(if preferences.movie_enabled {
                preferences.movie_volume
            } else {
                0
            })?,
        })
    }
}
/// This actor owns only native transport identifiers and clocks. Decoders,
/// decoded frames, PCM and media resource lifetime belong to CrySirror.
pub struct NativeMoviePlayer {
    client: ClientId,
    media: u32,
    layer: u32,
    size: [u32; 2],
    frame_ms: u64,
    pub spec: NativeMovieSpec,
    started: bool,
    visible: bool,
    loading: bool,
    path: Option<String>,
    exit_ms: Option<u64>,
    finished: bool,
    pub api_calls: u64,
}
impl NativeMoviePlayer {
    pub(crate) fn cancel(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.visible {
            host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(
                    &serde_json::json!({"layers":[{"op":"remove","id":self.layer}]}),
                )?,
            )?;
            self.visible = false;
            self.api_calls += 1;
        }
        if self.started && self.path.is_some() {
            host.media_request(self.client, MediaRequest::Remove { id: self.media })?;
            self.api_calls += 1;
        }
        self.path = None;
        self.loading = false;
        self.finished = true;
        Ok(())
    }
    pub fn new(
        client: ClientId,
        media: u32,
        layer: u32,
        size: [u32; 2],
        frame_ms: u64,
        spec: NativeMovieSpec,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0
                && media != 0
                && layer != 0
                && size.iter().all(|n| (1..=8192).contains(n))
                && (1..=10000).contains(&frame_ms),
            "NNN outer movie identities/bounds"
        );
        Ok(Self {
            client,
            media,
            layer,
            size,
            frame_ms,
            spec,
            started: false,
            visible: false,
            loading: false,
            path: None,
            exit_ms: None,
            finished: false,
            api_calls: 0,
        })
    }
    pub fn dependency_pending(&self) -> bool {
        self.loading && self.exit_ms.is_none() && !self.finished
    }
    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<Option<NativeMode>> {
        if self.finished {
            return Ok(Some(self.spec.next));
        }
        if !self.started {
            if let Some(asset) = &self.spec.asset {
                let path = host.media_request(
                    self.client,
                    MediaRequest::Play {
                        id: self.media,
                        asset: asset.clone(),
                        looping: false,
                        audio: true,
                        gain: self.spec.gain,
                    },
                )?;
                self.path = Some(path.relative().to_owned());
                self.loading = true;
                self.api_calls += 1;
            }
            self.started = true;
        }
        if self.spec.asset.is_none() {
            self.finished = true;
            return Ok(Some(self.spec.next));
        }
        if let Some(start) = self.exit_ms {
            ensure!(
                host.now_ms() >= start,
                "NNN movie exit clock moved backwards"
            );
            if host.now_ms() - start >= u64::from(self.spec.last_wait.max(1)) * self.frame_ms {
                self.finished = true;
                return Ok(Some(self.spec.next));
            }
            return Ok(None);
        }
        let info = host.media_info(self.client, self.media)?;
        self.loading = info.phase == MediaPhase::Loading;
        ensure!(
            info.phase != MediaPhase::Failed,
            "NNN outer movie shared decoder/transport failed"
        );
        ensure!(
            info.phase != MediaPhase::Stopped,
            "NNN outer movie transport stopped before native completion"
        );
        if !self.visible && info.published_frames > 0 {
            ensure!(
                info.size.is_some(),
                "NNN movie frame published without dimensions"
            );
            let [w, h] = self.size;
            let path = self.path.as_ref().context("NNN media surface absent")?;
            host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.layer,"rect":[0,0,w,h],"order":[3001,0],"paint":{"kind":"image","asset":path}}}]}))?)?;
            self.api_calls += 1;
            self.visible = true;
        }
        if info.phase == MediaPhase::Ended
            || (!self.spec.no_skip
                && input.buttons_pressed
                    & (crysirror_core::service::buttons::MOUSE_LEFT
                        | crysirror_core::service::buttons::MOUSE_RIGHT)
                    != 0)
        {
            if self.visible {
                host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(
                        &serde_json::json!({"layers":[{"op":"remove","id":self.layer}]}),
                    )?,
                )?;
                self.api_calls += 1;
                self.visible = false;
            }
            host.media_request(self.client, MediaRequest::Remove { id: self.media })?;
            self.api_calls += 1;
            self.path = None;
            self.exit_ms = Some(host.now_ms());
        }
        Ok(None)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"native_mode":"Movie","skipped_by_original_variable":self.spec.asset.is_none(),"asset":self.spec.asset,"started":self.started,"presented":self.visible,"exit_started_ms":self.exit_ms,"finished":self.finished,"engine_api_calls":self.api_calls})
    }
}
