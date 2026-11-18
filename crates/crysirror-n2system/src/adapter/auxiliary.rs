// SPDX-License-Identifier: MPL-2.0
//! Auxiliary NSS contracts from NitroSharp 77a93db4, VM/BuiltInFunctions.cs,
//! BuiltInFunctionDispatcher.cs and Rebuild/World.cs. Twitter uses that runtime's
//! explicit offline results, not fabricated account or delivery success.
use super::*;

impl N2SystemVm {
    pub(super) fn native_auxiliary(
        &mut self,
        host: &mut dyn HostServices,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        let result = match call.function {
            // These are the reference dispatcher's Windows-content profile,
            // not an Xbox host, authentication, achievement or disk-quota API.
            // Empty commands remain explicit; profile-dependent constants must
            // never be mistaken for observations from proprietary services.
            Builtin::XBOX360_LockVideo => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                let _ = args.truth(0)?;
                Value::Null
            }
            Builtin::XBOX360_Presence | Builtin::XBOX360_Achieved => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                let _ = args.integer(0)?;
                Value::Null
            }
            Builtin::XBOX360_AwardGameIcon | Builtin::XBOX360_InitUser => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                Value::Null
            }
            Builtin::XBOX360_IsAchieved => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                let _ = args.integer(0)?;
                Value::Boolean(true)
            }
            Builtin::XBOX360_IsSignin => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                Value::Boolean(true)
            }
            Builtin::XBOX360_CheckStorage | Builtin::XBOX360_SelectStorage => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                if call.function == Builtin::XBOX360_SelectStorage {
                    let _ = args.integer(0)?;
                }
                host.storage(self.client)?;
                Value::Boolean(true)
            }
            Builtin::XBOX360_StorageSize => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                Value::Number(i32::MAX as f32)
            }
            Builtin::XBOX360_UserIndex | Builtin::XBOX360_CurrentStorage => {
                ensure!(self.config.platform == 100, "Xbox service ABI profile");
                Value::Number(0.)
            }
            Builtin::XBOX360_PadTrigger => {
                use crysirror_core::service::{ControllerAxis, ControllerButton};
                let (axis, button) = match args.integer(0)? {
                    0 => (ControllerAxis::LeftTrigger, ControllerButton::LeftTrigger),
                    1 => (ControllerAxis::RightTrigger, ControllerButton::RightTrigger),
                    _ => bail!("N2System unknown trigger axis"),
                };
                let observed = host.input_sources(self.client)?;
                ensure!(
                    observed.client == self.client,
                    "N2System controller observation owner"
                );
                Value::Number(observed.sources.controllers.first().map_or(0., |pad| {
                    pad.buttons[button as usize]
                        .max(pad.axes[axis as usize].max(0.))
                        .clamp(0., 1.)
                }))
            }
            Builtin::log => {
                let message = args.string(0)?;
                ensure!(message.len() <= 16384, "N2System log byte limit");
                host.append_log(self.client, &message)?;
                Value::Null
            }
            Builtin::LegacyUnsupported => bail!(
                "N2System legacy source {}:{}: unsupported native command {}",
                args.string(0)?,
                args.integer(1)?,
                args.string(2)?,
            ),
            Builtin::Twitter_Init => {
                if !self.twitter_reported {
                    host.append_log(self.client,"N2System: Twitter integration is unavailable; account input and tweet requests return their offline results.")?;
                    self.twitter_reported = true;
                }
                Value::Null
            }
            Builtin::Twitter_CheckAcount | Builtin::Twitter_SetAcount => Value::Number(-1.),
            Builtin::Twitter_InputAcount | Builtin::Twitter_InputTweet | Builtin::Twitter_Tweet => {
                Value::Boolean(false)
            }
            Builtin::Reboot => {
                // Trial43CEB0 Reboot launches its restart helper, then485F70
                // terminates the process. Reconstruct this one portable game
                // session instead; do not execute a game-supplied EXE or retain
                // unsaved process globals as the ordinary Reset operation does.
                self.reboot_requested = true;
                self.reset_requested = true;
                state.thread.halted = true;
                state.wait = Wait::Frame(self.frame);
                Value::Number(0.)
            }
            Builtin::Reset => {
                // Reset is applied at the owner boundary, after the requesting
                // state is returned to the scheduler. No other script runs in
                // between; native variables/flags and the clock are retained.
                self.reset_requested = true;
                state.thread.halted = true;
                state.wait = Wait::Frame(self.frame);
                Value::Null
            }
            _ => bail!("N2System auxiliary dispatch mismatch"),
        };
        self.machine.complete(&mut state.thread, result)?;
        Ok(true)
    }

    pub(super) fn clear_native_world(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.cancel_modal_movie(host)?;
        self.modal_movie_results.clear();
        if self.native_dialog.take().is_some() {
            self.submit(host, vec![], vec![json!({"op":"close_dialog","id":1})])?;
        }
        self.native_dialog_results.clear();
        for ticket in std::mem::take(&mut self.voice_tickets) {
            let info = host.audio_preparation(self.client, ticket)?;
            host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel: info.channel,
                    fade_ms: 0.,
                },
            )?;
            host.release_audio(self.client, ticket)?;
        }
        for id in std::mem::take(&mut self.voices).into_values() {
            host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel: id,
                    fade_ms: 0.,
                },
            )?;
        }
        let roots = self
            .entities
            .nodes
            .values()
            .filter(|e| e.parent.is_none())
            .map(|e| e.id)
            .collect::<Vec<_>>();
        for id in roots {
            self.destroy(host, id)?;
        }
        self.video_capture = None;
        if let Some(path) = self.video_hold.take() {
            self.submit(
                host,
                vec![json!({"op":"remove","id":0x7fff_0001u32})],
                vec![],
            )?;
            host.release_image_resource(self.client, &path)?;
        }
        self.shortcuts.clear();
        self.shortcut_job = None;
        self.interrupt = None;
        self.history.clear();
        self.history_newline = false;
        self.history_revision = self
            .history_revision
            .checked_add(1)
            .context("backlog revision")?;
        self.hidden_for_text.clear();
        Ok(())
    }
    pub(super) fn reboot_native_session(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let client = self.client;
        let config = self.config.clone();
        // Allocate/validate the replacement before clearing the current world.
        let fresh = Self::new(client, config)?;
        self.clear_native_world(host)?;
        self.clear_preloaded(host)?;
        if let Some(library) = &self.fonts {
            for job in std::mem::take(&mut self.font_jobs) {
                library.release_job(job)?;
            }
            for font in std::mem::take(&mut self.font_handles) {
                library.release_font(font)?;
            }
        }
        // Dropping owner jobs cancels their old continuations. The fresh VM
        // performs ordinary asynchronous preparation and reads persisted common
        // data; no old module globals, arrays, clocks or RNG are transplanted.
        *self = fresh;
        Ok(())
    }
    pub(super) fn reset_native_world(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.clear_native_world(host)?;
        let main = self.entities.root("main".into())?;
        let thread = self
            .machine
            .start(&self.config.entry, None, "main".into())?;
        self.submit(host, vec![self.entity_group(main)?], vec![])?;
        self.main = main;
        self.threads.insert(main, Execution::new(thread));
        self.reset_requested = false;
        Ok(())
    }
}
