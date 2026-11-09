// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native semantic state and scoped owner calls. Layer/Canvas/Audio services
//! own all pixels, composition and playback. No original engine is hosted.
use crate::{
    machine::{Machine, NativeCall, Payload},
    metadata::Metadata,
    native_effects::{FadeInState, SunaParameters, SunaState},
    native_presentation::NativeFrameEffect,
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::image_processing::ImageResource;
use crysirror_core::{
    service::{
        AudioRequest, HostServices, InputFrame, PatternJob, PatternOutput, PatternProcessState,
        PatternRequest,
    },
    state::ClientId,
};
use crysirror_image::temporal_pattern::{PatternRng, TemporalPatternSpec};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

mod actor_frame;
mod message_skip;
mod save_state;
mod saved_effects;
mod saved_restore;
pub(crate) use saved_restore::BridgeRestoreSeed;

/// Explicit reviewed legacy callback ABI, not inferred from an arbitrary SPT.
/// The reviewed original module SHA and complete callback evidence are in SOURCE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackProfile {
    Mugen2Legacy,
}
/// Existing engine-owned scene root plus private adapter resource identifiers.
/// Caller initializes the root/backdrop before running this bounded profile.
#[derive(Clone, Copy, Debug)]
pub struct SceneBindings {
    pub root: u32,
    pub overlap_canvas: u32,
    pub effect_base: u32,
    pub size: [u32; 2],
    pub layers: u16,
    pub music_channel: u32,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum SavedWaitKind {
    Message,
    Selection,
}
pub struct Bridge {
    pub metadata: Metadata,
    pub api_calls: u64,
    pub overlap_captures: u64,
    pub option_off: i32,
    pub message_window_visible: bool,
    pub next_message_type: i32,
    pub next_window_mode: i32,
    pub last_message_id: Option<i32>,
    last_select_id: Option<i32>,
    last_select_hero: i32,
    saved_remaining: i32,
    pub message_have_voice: bool,
    /// Native callback members179/180/181, not STATUS window words180/181.
    pub message_voice_length_ms: u32,
    load_voice_length_ms: u32,
    auto_message: i32,
    legacy_demo_flag: i32,
    script_cursor_number: i32,
    script_mouse_number: i32,
    script_window_number: i32,
    configuration_mask: i32,
    window_binding: Option<crate::native_windows::WindowBinding>,
    cannot_click: Option<bool>,
    cannot_skip: i32,
    message_skip: bool,
    control_was_down: bool,
    shift_message_only: bool,
    auto_kill_skip: bool,
    cannot_kill_skip: bool,
    f4_enabled: bool,
    f5_enabled: bool,
    /// true selects F5; held across actor/image worker completion.
    native_skip_key: Option<bool>,
    mark_skipped_messages: bool,
    client: ClientId,
    bindings: SceneBindings,
    active_effects: BTreeSet<i32>,
    previous_effects: BTreeMap<i32, i32>,
    inactive_effect_records: BTreeMap<i32, crate::native_save::SavedEffect>,
    profile: CallbackProfile,
    patterns: BTreeMap<i32, SunaState>,
    pattern_cache: Option<Arc<dyn crysirror_core::service::PatternResource>>,
    saved_suna: BTreeMap<i32, SunaParameters>,
    not_trans: BTreeMap<i32, bool>,
    pending_pattern: Option<(NativeCall, i32, SunaParameters, PatternJob)>,
    layer_visible: BTreeMap<i32, bool>,
    fades: BTreeMap<i32, FadeInState>,
    greys: BTreeMap<i32, crate::native_effects::LayerGreyState>,
    negas: BTreeMap<i32, crate::native_effects::LayerGreyState>,
    chara_trans: BTreeMap<i32, crate::native_chara_trans::CharaTransState>,
    old_lamps: BTreeMap<i32, crate::native_old_lamp::OldLampState>,
    buras: BTreeMap<i32, crate::native_bura::BuraState>,
    moves: BTreeMap<i32, crate::native_move::NativeMoveState>,
    scrolls: BTreeMap<i32, crate::native_effects_scroll::ScrollState>,
    fusumas: BTreeMap<i32, crate::native_effects_fusuma::FusumaState>,
    shakes: BTreeMap<i32, crate::native_effects_shake::ShakeState>,
    flashes: BTreeMap<i32, crate::native_effects_flash::FlashState>,
    mosaics: BTreeMap<i32, crate::native_effects_mosaic::MosaicState>,
    picture_effects: BTreeMap<i32, crate::native_picture_effect::PictureEffectState>,
    audio: Option<crate::audio_bridge::AudioBridge>,
    text: Option<crate::native_text::NativeTextBridge>,
    selection: Option<crate::native_ui::NativeSelectionBridge>,
    restored_wait: Option<SavedWaitKind>,
    overlap_wait: Option<crate::native_overlap::NativeOverlap>,
    draw_wait: Option<crate::native_draw::NativeDraw>,
    frame_canvas_created: bool,
    frame_time_ms: u64,
    default_frame_time_ms: u64,
    display_path: Option<String>,
    last_frame: Option<Arc<dyn ImageResource>>,
    message_background: Option<Arc<dyn ImageResource>>,
    post_text_path: Option<String>,
    overlap_frame: Option<Arc<dyn ImageResource>>,
    images: Option<crate::image_bridge::ImageBridge>,
    image_paths: BTreeMap<i32, String>,
    picture_assignments: BTreeSet<i32>,
    rectangles: BTreeMap<i32, crate::native_geometry::NativeRects>,
    fill_effects: BTreeMap<i32, [u8; 3]>,
    passive_counts: BTreeMap<i32, (i32, i32)>,
    actor_clock: crysirror_scene::animation::AdaptiveStepClock,
    actor_clock_at: Option<u64>,
    last_native_steps: u32,
    last_background_steps: u32,
    background_maximum_steps: u32,
    frame_admission: bool,
    native_input: InputFrame,
    actor_frame: Option<actor_frame::ActorFrame>,
    actor_input: Option<InputFrame>,
    actor_ready_for_input: bool,
    actor_refresh_required: bool,
    functions: Option<Arc<crate::config::Names>>,
    skip: crate::native_skip::NativeSkipPolicy,
    /// Some only for an explicitly constructed original CBackExec context.
    background_read_marks: Option<bool>,
    system_file: Option<crate::native_system_file::NativeSystemFile>,
    system_script: Option<i32>,
    must_set_film: bool,
    must_set_cg: bool,
    must_set_music: bool,
    off_layer_dont_set_cg: bool,
    scene_replay: bool,
    skip_film_enabled: bool,
    // Legacy MESSAGEEXPSTATUS only writes CCommonPrintMessage+4552.
    // Static instruction-displacement audit finds writes at4FE990,4FE950,
    //500C80,5028C0 and no reads. Keep the native latch without inventing a
    //new click wait from the commented-out later CalcuCutinMode source.
    obsolete_message_exp_wait: bool,
    system_result_extension: bool,
    result_data: Option<crate::native_result::NativeResultData>,
    pending_result_save: Option<crate::native_result::ResultWrite>,
    pending_system_save: Option<crate::native_system_file::NativeSystemSave>,
    native_layer_off: u32,
    launch_recorded: bool,
}
impl Bridge {
    pub(crate) fn initialize_outer_title(
        &mut self,
        host: &mut dyn HostServices,
        frame_ms: u64,
    ) -> Result<u32> {
        ensure!(
            !self.dependency_pending(),
            "NNN Title Init requires drained original dependencies"
        );
        let file = self
            .system_file
            .as_mut()
            .context("NNN Title original SystemFile absent")?;
        let count = if self.launch_recorded {
            u32::try_from(file.system_word(208)?)?
        } else {
            let count = file.record_launch()?;
            self.launch_recorded = true;
            count
        };
        self.initialize_outer_opening(host, frame_ms)?;
        let audio = self
            .audio
            .as_mut()
            .context("NNN Title native audio capability absent")?;
        let before = audio.api_calls;
        audio.initialize_outer_title(host)?;
        self.api_calls += audio.api_calls - before;
        self.skip.stop_f5();
        self.native_skip_key = None;
        self.option_off = 0;
        self.next_window_mode = 0;
        Ok(count)
    }
    pub(crate) fn validate_frame_resource(&self, image: &dyn ImageResource) -> Result<()> {
        ensure!(
            image.client() == self.client && image.size() == self.bindings.size,
            "NNN immutable frame resource scope"
        );
        Ok(())
    }
    pub(crate) fn set_outer_exit_frame(&mut self, image: Arc<dyn ImageResource>) -> Result<()> {
        self.validate_frame_resource(image.as_ref())?;
        ensure!(
            !self.dependency_pending(),
            "NNN mode exit frame requires drained dependencies"
        );
        self.last_frame = Some(image.clone());
        self.overlap_frame = Some(image);
        Ok(())
    }
    pub(crate) fn initialize_outer_opening(
        &mut self,
        host: &mut dyn HostServices,
        frame_ms: u64,
    ) -> Result<()> {
        ensure!(
            !self.dependency_pending(),
            "NNN Opening Init requires drained dependencies"
        );
        ensure!(
            (1..=10000).contains(&frame_ms),
            "NNN Opening default frame duration"
        );
        let layers = self
            .active_effects
            .iter()
            .map(|slot| Ok(json!({"op":"set_visible","id":self.layer_id(*slot)?,"visible":true})))
            .collect::<Result<Vec<_>>>()?;
        if !layers.is_empty() {
            self.submit(host, json!({"layers":layers}))?;
        }
        // SetLayerOff()'s actual default is zero: make every original eye
        // visible; it does not mean clearing images or initializing variables.
        self.layer_visible.clear();
        self.native_layer_off = 0;
        self.set_frame_time(frame_ms)?;
        Ok(())
    }
    pub(crate) fn configure_system_file(
        &mut self,
        system_file: crate::native_system_file::NativeSystemFile,
        setup: &crate::config::Names,
    ) -> Result<()> {
        ensure!(
            self.system_file.is_none(),
            "NNN SystemFile already prepared"
        );
        let default = u64::try_from(setup.integer(b"frameTime")?.unwrap_or(50))?;
        ensure!(
            (1..=60000).contains(&default),
            "NNN default native frame period"
        );
        self.default_frame_time_ms = default;
        // Actual constructor0x542180: global max defaults2/min1, horizon10.
        // CheckNoSkipFrame0x535090 reads SystemFile object+220, serialized152.
        let configured_max = setup.integer(b"maxSkipFrame")?.unwrap_or(2).max(1);
        let background_maximum = setup
            .integer(b"maxSkipFrameBackScript")?
            .unwrap_or(configured_max);
        ensure!(
            (0..=1024).contains(&background_maximum),
            "NNN background scheduler limit outside shared step domain"
        );
        let maximum = if system_file.system_word(152)? != 0 {
            1
        } else {
            configured_max
        };
        let horizon = setup.integer(b"skipAdjustFrame")?.unwrap_or(10);
        let clock = crysirror_scene::animation::AdaptiveStepClock::new(
            self.frame_time_ms.try_into()?,
            maximum.try_into()?,
            horizon.try_into()?,
        )?;
        self.actor_clock = clock;
        self.background_maximum_steps = background_maximum as u32;
        self.actor_clock_at = None;
        self.must_set_film = setup.integer(b"mustSetFilmFlag")?.unwrap_or(1) != 0;
        self.must_set_cg = setup.integer(b"mustSetCGFlag")?.unwrap_or(1) != 0;
        self.must_set_music = setup.integer(b"mustSetMusicFlag")?.unwrap_or(1) != 0;
        self.off_layer_dont_set_cg = setup.integer(b"offLayerDontSetCG")?.unwrap_or(0) != 0;
        self.shift_message_only = setup.integer(b"shiftSkipMessageOnly")?.unwrap_or(0) != 0;
        self.auto_kill_skip = setup.integer(b"autoMessageCheckKillShift")?.unwrap_or(0) != 0;
        self.cannot_kill_skip = setup.integer(b"cannotSkipCheckKillShift")?.unwrap_or(0) != 0;
        self.f4_enabled = setup.integer(b"F4OK")?.unwrap_or(1) != 0;
        self.f5_enabled = setup.integer(b"F5Ok")?.unwrap_or(1) != 0;
        self.mark_skipped_messages = setup.integer(b"mustSetMessageFlag")?.unwrap_or(1) != 0;
        self.cannot_click = Some(false);
        self.system_result_extension = setup.integer(b"resultFlag")?.unwrap_or(0) != 0;
        self.system_file = Some(system_file);
        Ok(())
    }
    pub(crate) fn install_saved_wait(
        &mut self,
        prepared: crate::native_save_wait::PreparedWait,
    ) -> Result<(Value, Vec<String>)> {
        ensure!(
            prepared.text.is_some() != prepared.selection.is_some(),
            "NNN saved wait must own one native actor"
        );
        if let Some(text) = prepared.text {
            self.install_saved_text(text)?;
        }
        if let Some(selection) = prepared.selection {
            self.install_saved_selection(selection)?;
        }
        Ok((prepared.batch, prepared.release_images))
    }
    pub(crate) fn text_restore_seed(&self) -> Result<crate::native_text::TextRestoreSeed> {
        ensure!(
            !self.dependency_pending(),
            "NNN text restore requires drained native dependencies"
        );
        self.text
            .as_ref()
            .context("NNN original text component absent")?
            .restore_seed()
    }
    pub(crate) fn selection_restore_seed(&self) -> Result<crate::native_ui::SelectionRestoreSeed> {
        ensure!(
            !self.dependency_pending(),
            "NNN selection restore requires drained native dependencies"
        );
        self.selection
            .as_ref()
            .context("NNN original selection component absent")?
            .restore_seed()
    }
    pub(crate) fn install_saved_text(
        &mut self,
        text: crate::native_text::NativeTextBridge,
    ) -> Result<()> {
        ensure!(
            self.restored_wait.is_none()
                && text.profile.client == self.client
                && text.profile.root == self.bindings.root,
            "NNN saved text capability bindings"
        );
        self.text = Some(text);
        self.restored_wait = Some(SavedWaitKind::Message);
        Ok(())
    }
    pub(crate) fn install_saved_selection(
        &mut self,
        selection: crate::native_ui::NativeSelectionBridge,
    ) -> Result<()> {
        ensure!(
            self.restored_wait.is_none()
                && selection.profile.client == self.client
                && selection.profile.root == self.bindings.root,
            "NNN saved selection capability bindings"
        );
        self.selection = Some(selection);
        self.restored_wait = Some(SavedWaitKind::Selection);
        Ok(())
    }
    pub(crate) fn saved_wait_pending(&self) -> bool {
        self.restored_wait.is_some()
    }
    pub(crate) fn poll_saved_frame_dependency(
        &mut self,
        machine: &mut Machine,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        if self.actor_frame.is_some() {
            let selection = matches!(self.restored_wait, Some(SavedWaitKind::Selection));
            self.poll_message_frame(machine, None, selection, host)?;
        }
        Ok(())
    }
    pub(crate) fn poll_saved_wait(
        &mut self,
        machine: &mut Machine,
        program: &crate::Program,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool> {
        let message = matches!(self.restored_wait, Some(SavedWaitKind::Message));
        let selection = matches!(self.restored_wait, Some(SavedWaitKind::Selection))
            && self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.advances_effects());
        let animated =
            self.has_animated_effects() || self.actor_frame.is_some() || self.actor_ready_for_input;
        if message || selection {
            if message && animated {
                let retained = self.actor_input.get_or_insert(input);
                retained.buttons_pressed |= input.buttons_pressed;
            }
            if !self.poll_message_frame(machine, None, selection, host)? {
                return Ok(false);
            }
            if !animated {
                if selection {
                    self.advance_selection_actor_clock(host.now_ms())?;
                } else {
                    self.advance_static_actor_clock(host.now_ms())?;
                }
            }
        }
        let input = self.actor_input.take().unwrap_or(input);
        let automatic = self
            .system_file
            .as_ref()
            .map(|file| file.system_word(164))
            .transpose()?
            .unwrap_or(0)
            != 0;
        //53b630 restores raw lastMessageID28 independently of the old
        //CCommonPrintMessage display tag, which it does not overwrite.
        //Read/skip policy follows the restored reader and that raw ID.
        let serial = self.last_message_id.map_or(u32::MAX, |id| id as u32);
        let skip_allowed = message && self.native_message_skip_allowed(serial)?;
        if let Some(text) = self.text.as_mut() {
            text.system_auto(automatic);
            text.set_native_skip(skip_allowed);
        }
        if message && self.apply_native_skip_key(host)? {
            self.text
                .as_mut()
                .unwrap()
                .finish_native_action(machine, None, host.now_ms())?;
            self.restored_wait = None;
            self.actor_ready_for_input = false;
            return Ok(true);
        }
        let complete = match self.restored_wait {
            None => return Ok(true),
            Some(SavedWaitKind::Message) => self
                .text
                .as_mut()
                .context("NNN saved text absent")?
                .poll_restored_input(host, input)?,
            Some(SavedWaitKind::Selection) => self
                .selection
                .as_mut()
                .context("NNN saved selection absent")?
                .poll_restored_input(machine, program, host)?,
        };
        self.actor_ready_for_input = false;
        if complete {
            if message {
                self.mark_native_message_read(serial)?;
            } else {
                self.actor_refresh_required = true;
            }
            self.restored_wait = None;
        }
        if self
            .text
            .as_mut()
            .is_some_and(|text| text.take_disable_auto_request())
        {
            if let Some(file) = self.system_file.as_mut() {
                file.set_auto_mode(false)?;
            }
        }
        Ok(complete)
    }
    pub(crate) fn audio_restore_seed(&self) -> Result<crate::audio_bridge::AudioRestoreSeed> {
        ensure!(
            !self.dependency_pending(),
            "NNN restore seed requires drained native dependencies"
        );
        self.audio
            .as_ref()
            .context("NNN original audio component absent")?
            .restore_seed()
    }
    pub(crate) fn original_audio_work(&self) -> Result<crate::native_save_audio::SavedAudioWork> {
        self.audio
            .as_ref()
            .context("NNN original audio component absent")?
            .original_audio_work()
    }
    pub(crate) fn shutdown_audio(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(audio) = &mut self.audio {
            let before = audio.api_calls;
            audio.shutdown(host)?;
            self.api_calls += audio.api_calls - before;
        }
        Ok(())
    }
    pub(crate) fn system_file_mut(
        &mut self,
    ) -> Result<&mut crate::native_system_file::NativeSystemFile> {
        self.system_file
            .as_mut()
            .context("NNN original SystemFile not prepared")
    }
    pub(crate) fn set_system_script(&mut self, script: Option<u16>) {
        self.system_script = script.map(i32::from);
    }
    pub(crate) fn configure_system_sounds(
        &mut self,
        names: Option<crate::config::Names>,
        disabled: bool,
        multi: bool,
        base: u32,
    ) -> Result<()> {
        self.audio
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN native audio capability absent"))?
            .configure_system_sounds(names, disabled, multi, base)
    }
    pub(crate) fn poll_system_sound(
        &mut self,
        host: &mut dyn HostServices,
        number: i32,
    ) -> Result<bool> {
        let audio = self
            .audio
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN native system audio capability absent"))?;
        let before = audio.api_calls;
        let result = audio.poll_system_sound(host, number);
        self.api_calls += audio.api_calls - before;
        result
    }
    pub(crate) fn poll_outer_music(
        &mut self,
        host: &mut dyn HostServices,
        words: [i32; 4],
        restart: bool,
    ) -> Result<bool> {
        let audio = self
            .audio
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN native outer music capability not configured"))?;
        let before = audio.api_calls;
        let result = audio.poll_outer_music(host, words, restart);
        self.api_calls += audio.api_calls - before;
        result
    }
    pub(crate) fn presented_frame(&self) -> Option<Arc<dyn ImageResource>> {
        self.draw_wait
            .as_ref()
            .and_then(|draw| draw.presented_frame())
            .or_else(|| {
                self.overlap_wait
                    .as_ref()
                    .and_then(|overlap| overlap.presented_frame())
            })
            .or_else(|| self.last_frame.clone())
    }
    fn retire_post_text(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(path) = self.post_text_path.as_ref() {
            let id = self.bindings.effect_base + u32::from(self.bindings.layers) + 1;
            host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":id}]}))?,
            )?;
            host.release_image_resource(self.client, path)?;
            self.api_calls += 2;
            self.post_text_path = None;
        }
        Ok(())
    }
    pub(crate) fn stop_for_outer_movie(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(audio) = &mut self.audio {
            let before = audio.api_calls;
            audio.stop_for_outer_movie(host)?;
            self.api_calls += audio.api_calls - before;
        }
        Ok(())
    }
    /// Reviewed legacy InitNewGame presentation reset. Pixel caches are kept
    /// separately from original effect assignments, just like ClearAllEffect(FALSE).
    pub(crate) fn initialize_outer_newgame(
        &mut self,
        host: &mut dyn HostServices,
        exit: Arc<dyn ImageResource>,
    ) -> Result<()> {
        ensure!(
            exit.client() == self.client && exit.size() == self.bindings.size,
            "NNN NewGame exit frame scope"
        );
        ensure!(
            !self.dependency_pending() && self.draw_wait.is_none() && self.overlap_wait.is_none(),
            "NNN NewGame requires quiescent background dependencies"
        );
        let mut ops = self
            .active_effects
            .iter()
            .map(|slot| Ok(json!({"op":"remove","id":self.layer_id(*slot)?})))
            .collect::<Result<Vec<_>>>()?;
        if self.display_path.is_some() {
            ops.push(json!({"op":"remove","id":self.bindings.effect_base + u32::from(self.bindings.layers)}));
        }
        if self.post_text_path.is_some() {
            ops.push(json!({"op":"remove","id":self.bindings.effect_base+u32::from(self.bindings.layers)+1}));
        }
        if let Some(text) = &self.text {
            ops.extend(text.hide_operations()?);
        }
        self.submit(host, json!({"layers":ops}))?;
        if let Some(text) = &mut self.text {
            text.accept_hide()?;
            text.state.window_visible = false;
        }
        if let Some(path) = self.display_path.take() {
            host.release_image_resource(self.client, &path)?;
        }
        if let Some(path) = self.post_text_path.take() {
            host.release_image_resource(self.client, &path)?;
        }
        self.message_background = None;
        for (_, path) in std::mem::take(&mut self.image_paths) {
            host.release_image_resource(self.client, &path)?;
        }
        self.active_effects.clear();
        self.inactive_effect_records.clear();
        self.patterns.clear();
        self.fades.clear();
        self.greys.clear();
        self.negas.clear();
        self.chara_trans.clear();
        self.old_lamps.clear();
        self.buras.clear();
        self.moves.clear();
        self.scrolls.clear();
        self.fusumas.clear();
        self.shakes.clear();
        self.flashes.clear();
        self.picture_effects.clear();
        self.mosaics.clear();
        self.fill_effects.clear();
        self.passive_counts.clear();
        self.picture_assignments.clear();
        self.rectangles.clear();
        self.not_trans.clear();
        self.layer_visible.clear();
        self.native_layer_off = 0;
        self.last_frame = Some(exit.clone());
        self.overlap_frame = Some(exit);
        self.message_window_visible = false;
        self.option_off = 0;
        self.skip.next_command = false;
        self.skip.effect_command = false;
        self.skip.film_end = false;
        self.skip_film_enabled = false;
        self.obsolete_message_exp_wait = false;
        self.native_skip_key = None;
        // Actual53a710 invokes52f710(0) before starting the fresh reader.
        self.legacy_demo_flag = 0;
        if let Some(file) = self.system_file.as_mut() {
            file.set_auto_mode(false)?;
        }
        self.enter_foreground_context();
        Ok(())
    }
    pub fn new(
        client: ClientId,
        profile: CallbackProfile,
        bindings: SceneBindings,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0 && bindings.root != 0 && bindings.overlap_canvas != 0,
            "System-NNN presentation identity"
        );
        ensure!(
            bindings.size.iter().all(|v| (1..=8192).contains(v)) && bindings.layers == 16,
            "System-NNN presentation bounds"
        );
        let end = bindings
            .effect_base
            .checked_add(u32::from(bindings.layers))
            .ok_or_else(|| anyhow::anyhow!("System-NNN effect identifiers"))?;
        let post = end
            .checked_add(1)
            .context("NNN post-text layer identifier overflow")?;
        ensure!(
            bindings.root != end && bindings.root != post,
            "NNN frame/root identifier collision"
        );
        ensure!(
            !(bindings.effect_base..end).contains(&bindings.root),
            "System-NNN root/effect collision"
        );
        Ok(Self {
            metadata: Metadata::default(),
            api_calls: 0,
            overlap_captures: 0,
            option_off: 0,
            message_window_visible: false,
            next_message_type: 3,
            next_window_mode: -1,
            last_message_id: None,
            last_select_id: None,
            last_select_hero: 0,
            saved_remaining: 30,
            message_have_voice: false,
            message_voice_length_ms: 7000,
            load_voice_length_ms: 7000,
            auto_message: 0,
            legacy_demo_flag: 0,
            // The legacy constructor omitted these members' initialization.
            // Define pre-setter values rather than serializing heap bytes.
            script_cursor_number: 0,
            script_mouse_number: 0,
            script_window_number: 0,
            configuration_mask: 0,
            window_binding: None,
            cannot_click: None,
            cannot_skip: 0,
            message_skip: false,
            control_was_down: false,
            shift_message_only: false,
            auto_kill_skip: false,
            cannot_kill_skip: false,
            f4_enabled: true,
            f5_enabled: true,
            native_skip_key: None,
            mark_skipped_messages: true,
            client,
            bindings,
            active_effects: BTreeSet::new(),
            previous_effects: BTreeMap::new(),
            inactive_effect_records: BTreeMap::new(),
            profile,
            patterns: BTreeMap::new(),
            pattern_cache: None,
            saved_suna: BTreeMap::new(),
            not_trans: BTreeMap::new(),
            pending_pattern: None,
            layer_visible: BTreeMap::new(),
            fades: BTreeMap::new(),
            greys: BTreeMap::new(),
            negas: BTreeMap::new(),
            chara_trans: BTreeMap::new(),
            old_lamps: BTreeMap::new(),
            buras: BTreeMap::new(),
            moves: BTreeMap::new(),
            scrolls: BTreeMap::new(),
            fusumas: BTreeMap::new(),
            shakes: BTreeMap::new(),
            flashes: BTreeMap::new(),
            picture_effects: BTreeMap::new(),
            mosaics: BTreeMap::new(),
            audio: None,
            text: None,
            selection: None,
            restored_wait: None,
            overlap_wait: None,
            draw_wait: None,
            frame_canvas_created: false,
            frame_time_ms: 50,
            default_frame_time_ms: 50,
            display_path: None,
            last_frame: None,
            message_background: None,
            post_text_path: None,
            overlap_frame: None,
            images: None,
            image_paths: BTreeMap::new(),
            picture_assignments: BTreeSet::new(),
            rectangles: BTreeMap::new(),
            fill_effects: BTreeMap::new(),
            passive_counts: BTreeMap::new(),
            actor_clock: crysirror_scene::animation::AdaptiveStepClock::new(50, 2, 10)?,
            actor_clock_at: None,
            last_native_steps: 0,
            // CBackExec's constructor omitted m_kaeshita initialization.
            // Define its initial direct-DRAW0 CountUp as zero, not heap data.
            last_background_steps: 0,
            background_maximum_steps: 2,
            frame_admission: true,
            native_input: InputFrame::default(),
            actor_frame: None,
            actor_input: None,
            actor_ready_for_input: false,
            actor_refresh_required: false,
            functions: None,
            skip: crate::native_skip::NativeSkipPolicy::default(),
            background_read_marks: None,
            system_file: None,
            system_script: None,
            must_set_film: true,
            must_set_cg: true,
            must_set_music: true,
            off_layer_dont_set_cg: false,
            scene_replay: false,
            skip_film_enabled: false,
            obsolete_message_exp_wait: false,
            system_result_extension: false,
            result_data: None,
            pending_result_save: None,
            pending_system_save: None,
            native_layer_off: 0,
            launch_recorded: false,
        })
    }
    /// Configuration tables are native byte-name/number policy, never an
    /// arbitrary host callback registry. Unknown names still fail at their PC.
    pub fn configure_functions(&mut self, functions: crate::config::Names) -> Result<()> {
        ensure!(
            self.functions.is_none(),
            "NNN function policy already configured"
        );
        ensure!(
            functions.tokens.len() % 2 == 0,
            "NNN function name/label pairs"
        );
        self.functions = Some(Arc::new(functions));
        Ok(())
    }
    pub fn configure_background_read_marks(&mut self, allow: bool) -> Result<()> {
        ensure!(
            self.background_read_marks.is_none(),
            "NNN background read-mark policy already configured"
        );
        self.background_read_marks = Some(allow);
        Ok(())
    }
    pub(crate) fn enter_foreground_context(&mut self) {
        self.background_read_marks = None;
    }
    fn user_function_name(&self, call: &NativeCall) -> Result<&[u8]> {
        let table = self
            .functions
            .as_ref()
            .context("NNN original user-function table not configured")?;
        let at = (call.code as usize)
            .checked_mul(2)
            .context("NNN function index overflow")?;
        Ok(table
            .tokens
            .get(at)
            .context("NNN original function index outside table")?)
    }
    /// Original calculated expmode/expcheck groups read SystemFile preferences,
    /// not their ordinary script words. The system file is owner-held and only
    /// native callbacks/UI may mutate it, so one frozen VM admission is stable.
    pub(crate) fn bind_preference_variables(
        &self,
        variables: &mut crate::expression::Variables,
    ) -> Result<()> {
        let Some(file) = &self.system_file else {
            return Ok(());
        };
        // Legacy constructor binds only the first five names of each group,
        // although the stored preference arrays have sixteen entries each.
        for (prefix, offset) in [("expmode", 1792), ("expcheck", 1856)] {
            for index in 0..5 {
                variables.bind_calculated_value(
                    format!("{prefix}{}", index + 1).as_bytes(),
                    file.system_word(offset + index * 4)?,
                )?;
            }
        }
        Ok(())
    }
    fn try_system_save(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        ensure!(
            !self.system_result_extension || self.result_data.is_some(),
            "NNN savesystemfile original ResultData was not prepared"
        );
        ensure!(
            self.system_file.is_some(),
            "NNN savesystemfile original SystemFile not prepared"
        );
        if self.pending_system_save.is_none() {
            let variables = machine.native_variables_candidate(call)?;
            let work = crate::native_system_file::NativeSystemSave::begin(
                self.client,
                call,
                variables,
                host,
            )?;
            self.api_calls += work.api_calls;
            self.pending_system_save = Some(work);
        }
        let work = self.pending_system_save.as_mut().unwrap();
        let before = work.api_calls;
        let result = work.poll_storage(self.system_file.as_mut().unwrap(), call, host);
        self.api_calls += work.api_calls - before;
        if result? {
            if !self.poll_result_save(host)? {
                return Ok(false);
            }
            self.pending_system_save
                .as_ref()
                .unwrap()
                .acknowledge(machine, call)?;
            self.pending_system_save = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub(crate) fn configure_result_data(
        &mut self,
        data: Option<crate::native_result::NativeResultData>,
    ) -> Result<()> {
        ensure!(
            self.result_data.is_none() && self.pending_result_save.is_none(),
            "NNN result data already configured"
        );
        ensure!(
            self.system_result_extension == data.is_some(),
            "NNN result configuration does not match native resultFlag"
        );
        self.result_data = data;
        Ok(())
    }
    pub(crate) fn poll_result_save(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if !self.system_result_extension {
            return Ok(true);
        }
        let data = self
            .result_data
            .as_mut()
            .context("NNN native ResultData absent")?;
        if self.pending_result_save.is_none() {
            self.pending_result_save = Some(data.begin_save(host.storage(self.client)?.as_ref())?);
            self.api_calls += 1;
        }
        if !self.pending_result_save.as_mut().unwrap().poll(data)? {
            return Ok(false);
        }
        self.pending_result_save = None;
        Ok(true)
    }
    /// Native config dialogue policy (401ea0/402b00), not host UI flags.
    pub fn configuration_mask(&self) -> i32 {
        self.configuration_mask
    }
    fn tagged_message_serial(reader: Option<i32>, raw: u32) -> u32 {
        if raw & 0xffff == 0xffff {
            u32::MAX
        } else {
            ((reader.unwrap_or(0) as u32) << 16) | (raw & 0xffff)
        }
    }
    pub(crate) fn set_frame_admission(&mut self, admit: bool) {
        self.frame_admission = admit;
    }
    fn draw_skip_requested(&self) -> bool {
        if self.message_skip && self.auto_message <= 0 && self.cannot_skip == 0 {
            return true;
        }
        self.auto_message <= 0
            && !self.cannot_click.unwrap_or(false)
            && self
                .text
                .as_ref()
                .is_some_and(|text| text.draw_skip_click(self.native_input))
    }
    pub(crate) fn configure_window_binding(
        &mut self,
        services: crate::native_services_bootstrap::PreparedNativeServices,
        resources: crate::resource_bridge::NativeResources,
        levels: u32,
        pool: crysirror_tasks::TaskPool,
    ) -> Result<()> {
        ensure!(
            self.window_binding.is_none(),
            "NNN window policy already configured"
        );
        let text = self
            .text
            .as_ref()
            .context("NNN window policy requires native text")?;
        let ids = [
            text.profile
                .window
                .as_ref()
                .map_or(5002, |window| window.id),
            text.profile
                .name_window
                .as_ref()
                .map_or(5003, |window| window.id),
        ];
        self.window_binding = Some(crate::native_windows::WindowBinding::new(
            services, resources, levels, pool, ids,
        ));
        Ok(())
    }
    pub(crate) fn prepare_native_windows(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let Some(binding) = self.window_binding.as_mut() else {
            ensure!(
                self.script_window_number == 0,
                "NNN window variant capability not configured"
            );
            return Ok(true);
        };
        if binding.loaded(self.script_window_number) {
            return Ok(true);
        }
        let before = binding.api_calls;
        let ready = binding.poll(self.client, self.script_window_number, host);
        self.api_calls += binding.api_calls - before;
        let Some(windows) = ready? else {
            return Ok(false);
        };
        self.text
            .as_mut()
            .context("NNN native window text absent")?
            .bind_windows(self.script_window_number, windows)?;
        Ok(true)
    }
    pub(crate) fn sync_window_seed(
        &self,
        seed: &mut crate::native_text::TextRestoreSeed,
    ) -> Result<()> {
        let text = self
            .text
            .as_ref()
            .context("NNN native window text absent")?;
        seed.bind_windows(
            self.script_window_number,
            text.profile
                .window
                .iter()
                .chain(text.profile.name_window.iter())
                .cloned()
                .collect(),
        )
    }
    pub(crate) fn scene_replay_active(&self) -> bool {
        self.scene_replay
    }
    pub fn skip_policy(&self) -> &crate::native_skip::NativeSkipPolicy {
        &self.skip
    }
    fn message_control(&self) -> crate::native_message_control::NativeMessageControl {
        crate::native_message_control::NativeMessageControl {
            last_message_id: self.last_message_id,
            next_type: self.next_message_type,
            next_window: self.next_window_mode,
            window_visible: self.message_window_visible,
            has_voice: self.message_have_voice,
        }
    }
    fn adopt_message_control(
        &mut self,
        policy: crate::native_message_control::NativeMessageControl,
    ) {
        self.last_message_id = policy.last_message_id;
        self.next_message_type = policy.next_type;
        self.next_window_mode = policy.next_window;
        self.message_window_visible = policy.window_visible;
        self.message_have_voice = policy.has_voice;
    }
    /// Pending task readiness is independent from an original timer or input
    /// wait. This query never polls a receipt, executes code or consumes input.
    pub fn dependency_pending(&self) -> bool {
        self.actor_frame.is_some()
            || !self.saved_suna.is_empty()
            || self.pending_pattern.is_some()
            || self
                .window_binding
                .as_ref()
                .is_some_and(|binding| binding.pending())
            || self.pending_system_save.is_some()
            || self.pending_result_save.is_some()
            || self.images.as_ref().is_some_and(|s| s.dependency_pending())
            || self.audio.as_ref().is_some_and(|s| s.pending())
            || self.text.as_ref().is_some_and(|s| s.dependency_pending())
            || self
                .selection
                .as_ref()
                .is_some_and(|s| s.dependency_pending())
            || self
                .draw_wait
                .as_ref()
                .is_some_and(|s| s.dependency_pending())
            || self
                .overlap_wait
                .as_ref()
                .is_some_and(|s| s.dependency_pending())
    }
    pub(crate) fn presentation_pending(&self) -> bool {
        self.actor_refresh_required
            || self.actor_frame.is_some()
            || self
                .text
                .as_ref()
                .is_some_and(|text| text.dependency_pending())
            || self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.dependency_pending())
            || self
                .draw_wait
                .as_ref()
                .is_some_and(|draw| draw.dependency_pending())
            || self
                .overlap_wait
                .as_ref()
                .is_some_and(|overlap| overlap.dependency_pending())
    }
    pub fn wait_kind(&self, call: Option<&NativeCall>) -> &'static str {
        match self.restored_wait {
            Some(SavedWaitKind::Message) => return "saved_message_input",
            Some(SavedWaitKind::Selection) => return "saved_selection_input",
            None => {}
        }

        if self.dependency_pending() {
            return "resource";
        }
        if self.draw_wait.is_some() || self.overlap_wait.is_some() {
            return "timer";
        }
        if call.is_some_and(|call| {
            matches!(
                call.payload,
                Payload::Message { .. } | Payload::Selection { .. }
            )
        }) {
            return "input";
        }
        "owner"
    }
    pub fn previous_effect(&self, slot: i32) -> Option<i32> {
        self.previous_effects.get(&slot).copied()
    }
    pub fn configure_audio(&mut self, audio: crate::audio_bridge::AudioBridge) -> Result<()> {
        ensure!(
            audio.client() == self.client,
            "NNN audio bridge foreign client"
        );
        ensure!(self.audio.is_none(), "NNN audio bridge already configured");
        self.audio = Some(audio);
        Ok(())
    }
    pub fn pattern(&self, slot: i32) -> Option<&SunaState> {
        self.patterns.get(&slot)
    }
    pub fn picture(&self, slot: i32) -> Option<&crate::image_bridge::PreparedPicture> {
        self.images.as_ref()?.pictures.get(&slot)
    }
    pub fn set_frame_time(&mut self, value: u64) -> Result<()> {
        ensure!((1..=60000).contains(&value), "NNN original frameTime range");
        self.actor_clock.set_period(value.try_into()?)?;
        self.frame_time_ms = value;
        Ok(())
    }
    pub(crate) fn frame_period_ms(&self) -> u64 {
        self.frame_time_ms
    }
    pub(crate) fn passive_effect_counts(&self) -> &BTreeMap<i32, (i32, i32)> {
        &self.passive_counts
    }
    /// One real owner actor update; dependency-only pumps never call this.
    /// Static Nop/Fill have no Calcu pixel work, but CountUp is still native
    /// saved state and must not freeze merely because their pixels are static.
    fn advance_static_actor_clock(&mut self, now: u64) -> Result<()> {
        if self.actor_clock_at == Some(now) {
            return Ok(());
        }
        let elapsed = match self.actor_clock_at {
            Some(previous) => now
                .checked_sub(previous)
                .context("NNN native actor clock moved backwards")?,
            None => 0,
        };
        let mut clock = self.actor_clock.clone();
        clock.begin(elapsed.try_into()?)?;
        let steps = clock.available(None)?;
        clock.consume_frames(steps)?;
        clock.end()?;
        self.actor_clock = clock;
        self.actor_clock_at = Some(now);
        self.last_native_steps = steps;
        for (count, max) in self.passive_counts.values_mut() {
            if *count < *max {
                *count = count.saturating_add(steps as i32).min(*max);
            }
        }
        Ok(())
    }
    fn advance_selection_actor_clock(&mut self, now: u64) -> Result<()> {
        if self.actor_clock_at == Some(now) {
            return Ok(());
        }
        self.actor_clock.clear()?;
        self.actor_clock_at = Some(now);
        for (count, max) in self.passive_counts.values_mut() {
            if *count < *max {
                *count = count.saturating_add(1).min(*max);
            }
        }
        Ok(())
    }
    pub fn effect_counter(&self, slot: i32) -> Option<(i32, i32)> {
        self.passive_counts
            .get(&slot)
            .copied()
            .or_else(|| {
                self.patterns
                    .get(&slot)
                    .map(|s| (s.count, s.parameters.frames))
            })
            .or_else(|| self.fades.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.greys.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.negas.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.chara_trans.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.old_lamps.get(&slot).map(|s| (s.count, 999999)))
            .or_else(|| self.buras.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.moves.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.scrolls.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.fusumas.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.shakes.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.flashes.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.picture_effects.get(&slot).map(|s| (s.count, s.frames)))
            .or_else(|| self.mosaics.get(&slot).map(|s| (s.count, s.frames)))
    }
    pub fn configure_resources(
        &mut self,
        resources: crate::resource_bridge::NativeResources,
        names: crate::native_resources::ResourceNames,
        levels: u32,
    ) -> Result<()> {
        ensure!(self.images.is_none(), "NNN image bridge already configured");
        if let Some(selection) = self.selection.as_mut() {
            selection.configure_background_resources(resources.clone(), names.clone(), levels)?;
        }
        self.images = Some(crate::image_bridge::ImageBridge::new(
            self.client,
            names,
            levels,
            resources,
        )?);
        Ok(())
    }
    pub fn configure_text(&mut self, text: crate::native_text::NativeTextBridge) -> Result<()> {
        ensure!(
            self.text.is_none()
                && text.profile.client == self.client
                && text.profile.root == self.bindings.root,
            "NNN text bridge scope/already configured"
        );
        self.text = Some(text);
        Ok(())
    }
    pub fn configure_selection(
        &mut self,
        mut selection: crate::native_ui::NativeSelectionBridge,
    ) -> Result<()> {
        ensure!(
            self.selection.is_none()
                && selection.profile.client == self.client
                && selection.profile.root == self.bindings.root,
            "NNN selection bridge scope/already configured"
        );
        if let Some(images) = &self.images {
            let (names, levels) = images.naming_policy();
            selection.configure_background_resources(images.resources(), names, levels)?;
        }
        self.selection = Some(selection);
        Ok(())
    }
    fn bind_completed_foreground_mode(&self, machine: &mut Machine, mode: i32) -> Result<()> {
        if self.background_read_marks.is_none() {
            if let Some(current) = machine.variables() {
                let mut variables = current.clone();
                variables.bind_calculated_value(b"mode", mode)?;
                machine.replace_variables(variables)?;
            }
        }
        Ok(())
    }
    pub fn try_commit_with_context(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        program: &crate::Program,
        input: InputFrame,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        machine.validate_receipt(call)?;
        if matches!(call.payload, Payload::Message { .. }) {
            if self.skip.next_command {
                let text = self
                    .text
                    .as_mut()
                    .context("NNN skipped message actor absent")?;
                if !text.pending_message() {
                    let default = call.code == 0x22220001;
                    let visible = match self.next_window_mode {
                        0 => false,
                        1 => true,
                        _ => default,
                    };
                    text.set_next_window_visibility(visible)?;
                    self.next_window_mode = -1;
                }
                let complete = text.try_commit_skipped(machine, call, host)?;
                if complete {
                    let Payload::Message { serial, .. } = &call.payload else {
                        unreachable!()
                    };
                    text.state.serial = Self::tagged_message_serial(self.system_script, *serial);
                }
                self.message_window_visible = text.state.window_visible;
                if complete {
                    let Payload::Message { serial, .. } = &call.payload else {
                        unreachable!()
                    };
                    self.last_message_id = Some(*serial as i32);
                    if self.mark_skipped_messages {
                        self.mark_native_message_read(*serial)?;
                    }
                }
                return Ok(complete);
            }
            let animated_frame = self.has_animated_effects()
                || self.actor_frame.is_some()
                || self.actor_ready_for_input;
            let starting = !self
                .text
                .as_ref()
                .is_some_and(|text| text.pending_message());
            if starting && self.actor_frame.is_none() && !self.actor_ready_for_input {
                self.actor_clock_at = None;
            }
            let published = self
                .text
                .as_ref()
                .is_some_and(|text| text.published_message());
            if starting || published {
                if published && (self.has_animated_effects() || self.actor_frame.is_some()) {
                    let retained = self.actor_input.get_or_insert(input);
                    retained.buttons_pressed |= input.buttons_pressed;
                }
                if !self.poll_message_frame(machine, Some(call), false, host)? {
                    return Ok(false);
                }
            }
            let input = self.actor_input.take().unwrap_or(input);
            let system_auto = self
                .system_file
                .as_ref()
                .map(|file| file.system_word(164))
                .transpose()?
                .unwrap_or(0)
                != 0;
            let serial = match &call.payload {
                Payload::Message { serial, .. } => *serial,
                _ => unreachable!(),
            };
            let skip_allowed = self.native_message_skip_allowed(serial)?;
            let text = self
                .text
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("NNN native message profile not prepared"))?;
            text.set_native_skip(skip_allowed);
            if let Some(background) = self
                .message_background
                .as_ref()
                .or(self.last_frame.as_ref())
            {
                text.set_background(background.clone())?;
            }
            if !text.pending_message() {
                text.start_native_controls(
                    self.auto_message,
                    system_auto,
                    self.cannot_click.unwrap_or(text.profile.cannot_click),
                    self.message_have_voice,
                    self.message_voice_length_ms,
                )?;
                let default = call.code == 0x22220001;
                let visible = match self.next_window_mode {
                    0 => false,
                    1 => true,
                    _ => default,
                };
                text.set_next_window_visibility(visible)?;
                self.next_window_mode = -1;
                if let Payload::Message { serial, .. } = &call.payload {
                    self.last_message_id = Some(*serial as i32);
                }
            }
            let before = text.api_calls;
            text.system_auto(system_auto);
            let published = text.prepare_publication(machine, call, host)?;
            self.api_calls += text.api_calls - before;
            self.message_window_visible = text.state.window_visible;
            if !published {
                return Ok(false);
            }
            //538a70/5389c0/538910 pass reader-tagged serial to4ff500;
            //CGame.lastMessageID remains the unmodified script number.
            if let Payload::Message { serial, .. } = &call.payload {
                text.state.serial = Self::tagged_message_serial(self.system_script, *serial);
            }
            if self.actor_frame.is_some()
                && !self.poll_message_frame(machine, Some(call), false, host)?
            {
                return Ok(false);
            }
            if self.apply_native_skip_key(host)? {
                self.text.as_mut().unwrap().finish_native_action(
                    machine,
                    Some(call),
                    host.now_ms(),
                )?;
                self.actor_ready_for_input = false;
                self.bind_completed_foreground_mode(machine, 10)?;
                return Ok(true);
            }
            let text = self.text.as_mut().unwrap();
            let before = text.api_calls;
            let result = text.try_commit(machine, call, host, input);
            self.message_window_visible = text.state.window_visible;
            self.api_calls += text.api_calls - before;
            if text.take_disable_auto_request() {
                if let Some(file) = self.system_file.as_mut() {
                    file.set_auto_mode(false)?;
                }
            }
            let complete = result?;
            self.actor_ready_for_input = false;
            if !text.dependency_pending() && !animated_frame {
                self.advance_static_actor_clock(host.now_ms())?;
            }
            if complete {
                self.mark_native_message_read(serial)?;
                self.bind_completed_foreground_mode(machine, 10)?;
            }
            return Ok(complete);
        }
        if matches!(call.payload, Payload::Selection { .. }) {
            let animated = self.has_animated_effects()
                || self.actor_frame.is_some()
                || self.actor_ready_for_input;
            let animate_background = self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.advances_effects());
            let starting = !self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.pending_selection());
            if starting {
                if let Payload::Selection { resume_id, .. } = &call.payload {
                    self.last_select_id = Some(*resume_id as i32);
                }
                // Native SystemCommandSelect always stops script/film skip;
                // F4/F5 identity flags remain until their explicit ClearF5.
                self.skip.next_command = false;
                self.skip.effect_command = false;
                self.skip.film_end = false;
                self.skip_film_enabled = false;
                self.native_skip_key = None;
            }
            let published = self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.published_selection());
            if animate_background && (starting || published) {
                if starting && self.actor_frame.is_none() && !self.actor_ready_for_input {
                    self.actor_clock_at = None;
                }
                if !self.poll_message_frame(machine, Some(call), true, host)? {
                    return Ok(false);
                }
            }
            let selection = self
                .selection
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("NNN native selection profile not prepared"))?;
            let before = selection.api_calls;
            if !selection.pending_selection() {
                if !animated {
                    self.actor_clock_at = None;
                }
            }
            let result = selection.try_commit(machine, call, program, host);
            self.api_calls += selection.api_calls - before;
            let complete = result?;
            self.actor_ready_for_input = false;
            if selection.advances_effects()
                && selection.published_selection()
                && !selection.dependency_pending()
                && !animated
            {
                self.advance_selection_actor_clock(host.now_ms())?;
            }
            if complete {
                self.actor_refresh_required = true;
            }
            return Ok(complete);
        }
        self.try_commit(machine, call, host)
    }
    /// Nonblocking owner-side resource dependency. No native receipt or global
    /// random state is committed until the scoped engine worker has succeeded.
    pub fn try_commit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        machine.validate_receipt(call)?;
        if self.skip.next_command
            && self.skip.effect_command
            && ((call.identify == 0x66660007
                && matches!(call.code, 0x33330011 | 0x33330015 | 0x3333001f))
                || (call.identify == 0x66660006
                    && call.code == 0x22220005
                    && self.background_read_marks.is_none()))
        {
            // Actual538b20/541020/540f20 suppress resource/capture work before
            // touching its parameters. Actual53fdc0's back-reader branch is
            // intentionally excluded: it precedes the foreground skip test.
            machine.acknowledge(call)?;
            return Ok(true);
        }
        if call.identify == 0x66660007
            && call.code == 0x33330013
            && self.skip.next_command
            && self.skip.effect_command
        {
            // Native538670 returns before SetParam/clear when both script
            // skip flags are set; do not load an effect worker or draw RNG.
            machine.acknowledge(call)?;
            return Ok(true);
        }
        if let Some(work) = &self.pending_system_save {
            ensure!(
                work.matches(call),
                "NNN pending system-save owns another native receipt"
            );
        }
        if call.identify == 0x66660007 && call.code == 0x33330054 {
            let Payload::Scalar(value) = &call.payload else {
                bail!("NNN WindowNumber scalar ABI");
            };
            //5365d0 stores the raw CGame value; the next native window use
            //loads both sources. Admit that I/O as an owned dependency here,
            //without publishing or redrawing the retained old message.
            self.script_window_number = *value;
            if !self.prepare_native_windows(host)? {
                return Ok(false);
            }
            machine.acknowledge(call)?;
            return Ok(true);
        }
        if matches!(call.payload, Payload::UserCall { command: false, .. })
            && self
                .user_function_name(call)?
                .eq_ignore_ascii_case(b"savesystemfile")
        {
            return self.try_system_save(machine, call, host);
        }
        if call.identify == 0x66660002
            && matches!(
                call.payload,
                Payload::Control {
                    code: 0x5556000c | 0x5556000e | 0x55560013,
                    ..
                }
            )
        {
            if self.background_read_marks.is_some() {
                return self.try_draw(machine, call, host);
            }
            // Complete ordinary Yoyaku control family: these returns resume
            // immediately, without the CBackExec one-frame draw. Scene replay
            // has a different NextSceneFilm/return-mode contract and is not ACKed.
            if matches!(
                call.payload,
                Payload::Control {
                    code: 0x5556000c,
                    ..
                }
            ) {
                ensure!(
                    !self.scene_replay,
                    "NNN scene replay ENDFILM requires original NextSceneFilm mode"
                );
                if self.skip.film_end {
                    self.skip.film_end = false;
                    self.skip.next_command = false;
                    self.skip.effect_command = false;
                    self.skip_film_enabled = false;
                }
            }
            machine.acknowledge(call)?;
            return Ok(true);
        }
        if call.identify == 0x66660006 && call.code == 0x22220005 {
            return self.try_overlap(machine, call, host);
        }
        if call.identify == 0x66660006 && call.code == 0x22220004 {
            return self.try_draw(machine, call, host);
        }
        if matches!(call.payload, Payload::Image { .. }) {
            return self.try_image(machine, call, host);
        }
        if matches!(call.code, 0x33330007 | 0x33330008 | 0x33330009) {
            let context = crate::native_audio::AudioCallbackContext {
                auto_message: self.auto_message != 0,
                message_skipping: self.message_skip,
                suppress_voice: self.native_voice_suppressed()?,
                must_set_music: self.must_set_music,
            };
            if call.code == 0x33330007
                && matches!(&call.payload,Payload::Words{words,..} if words.first().is_some_and(|n|*n>0))
                && (self.must_set_music || !(self.skip.next_command && self.skip.effect_command))
            {
                ensure!(
                    self.system_file.is_some(),
                    "NNN music read-mark SystemFile absent"
                );
            }
            if let Some(audio) = self.audio.as_mut() {
                let before = audio.api_calls;
                let result = audio.try_commit_with_skip(machine, call, host, &self.skip, context);
                self.api_calls += audio.api_calls - before;
                let complete = result?;
                if complete {
                    if let Some((channel, length)) = audio.take_message_voice_start() {
                        if channel < 2 {
                            //531820 sets179/180/181 only for a real new voice,
                            // including muted decoding. Skip/stop has no receipt.
                            self.message_have_voice = true;
                            self.load_voice_length_ms = length;
                            self.message_voice_length_ms = length;
                        }
                    }
                    if let Some(number) = audio.take_music_read_mark() {
                        self.system_file
                            .as_mut()
                            .context("NNN music read-mark SystemFile absent")?
                            .mark_music(number)?;
                    }
                }
                return Ok(complete);
            }
        }
        if self.pending_pattern.is_none() {
            if let Payload::Words { count, words } = &call.payload {
                if call.code == 0x33330013 && *count >= 2 && words[1] == 29 {
                    let slot = words[0];
                    self.layer_id(slot)?;
                    let parameters = SunaParameters::parse(&words[2..], self.bindings.size)?;
                    let candidate = machine.native_variables_candidate(call)?;
                    let rng = candidate.random_state().ok_or_else(|| {
                        anyhow::anyhow!("SUNA requires original initialized shared random state")
                    })?;
                    let spec = TemporalPatternSpec {
                        size: self.bindings.size,
                        cell: [2, 2],
                    };
                    spec.validate()?;
                    let job = host.begin_pattern(
                        self.client,
                        PatternRequest::Create {
                            spec: serde_json::to_vec(&spec)?.into(),
                            rng: serde_json::to_vec(&rng)?.into(),
                        },
                    )?;
                    self.api_calls += 1;
                    self.pending_pattern = Some((call.clone(), slot, parameters, job));
                } else {
                    self.commit(machine, call, host)?;
                    return Ok(true);
                }
            } else {
                self.commit(machine, call, host)?;
                return Ok(true);
            }
        }
        let (receipt, slot, parameters, job) = self.pending_pattern.as_mut().unwrap();
        ensure!(
            receipt == call,
            "System-NNN resource receipt changed while pending"
        );
        match job.poll()? {
            PatternProcessState::Pending => Ok(false),
            PatternProcessState::Failed(error) => bail!("SUNA engine pattern creation: {error}"),
            PatternProcessState::Ready(PatternOutput::Resource {
                resource,
                rng_after,
            }) => {
                ensure!(
                    resource.client() == self.client && resource.info().size == self.bindings.size,
                    "SUNA scoped engine resource"
                );
                let slot = *slot;
                let parameters = parameters.clone();
                let rng: PatternRng = serde_json::from_slice(&rng_after)?;
                rng.validate()?;
                let mut candidate = machine.native_variables_candidate(call)?;
                candidate.replace_random_state(rng)?;
                self.restore_picture_source(slot, host)?;
                machine.acknowledge_with_variables(call, candidate)?;
                self.fill_effects.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.passive_counts.remove(&slot);
                // Native CAllEffect owns one CEffectSuna object shared by all
                // effect29 layers. SetParam reinitializes its one pattern,
                // rather than giving every layer a private random texture.
                for state in self.patterns.values_mut() {
                    state.resource = resource.clone();
                }
                self.pattern_cache = Some(resource.clone());
                self.patterns.insert(
                    slot,
                    SunaState {
                        parameters,
                        resource,
                        count: 0,
                    },
                );
                self.previous_effects.insert(slot, 29);
                self.pending_pattern = None;
                Ok(true)
            }
            PatternProcessState::Ready(_) => bail!("SUNA engine returned image for creation"),
        }
    }
    fn try_image(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        let images = self
            .images
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN scoped image resources not prepared"))?;
        let before = images.api_calls;
        let result = images.prepare(machine, call, host);
        self.api_calls += images.api_calls - before;
        let Some(picture) = result? else {
            return Ok(false);
        };
        if !picture.preload {
            let id = self.layer_id(picture.slot)?;
            let size = picture.display.size();
            let rectangles = crate::native_geometry::NativeRects::natural(size);
            let opaque = self
                .not_trans
                .get(&picture.slot)
                .copied()
                .unwrap_or(picture.slot < 4 || picture.slot == 14);
            let image = if opaque {
                picture.opaque.clone()
            } else if picture.has_mask {
                picture.raw.clone()
            } else {
                picture.keyed.clone()
            };
            let published = host.publish_image_resource(self.client, id, image)?;
            self.api_calls += 1;
            if !self.fill_effects.contains_key(&picture.slot) {
                let op = self.picture_operation(
                    picture.slot,
                    size,
                    published.path.relative(),
                    &rectangles,
                )?;
                if let Err(error) = self.submit(host, json!({"layers":[op]})) {
                    let _ = host.release_image_resource(self.client, published.path.relative());
                    return Err(error);
                }
            }
            machine.acknowledge(call)?;
            self.images.as_mut().unwrap().accept(call)?;
            if let Some(previous) = self
                .image_paths
                .insert(picture.slot, published.path.relative().to_owned())
            {
                host.release_image_resource(self.client, &previous)?;
            }
            self.active_effects.insert(picture.slot);
            self.picture_assignments.insert(picture.slot);
            self.not_trans
                .entry(picture.slot)
                .or_insert(picture.slot < 4 || picture.slot == 14);
            self.rectangles.insert(picture.slot, rectangles);
        } else if self.picture_assignments.contains(&picture.slot) {
            // Actual540f20 replaces CPicture without invoking SetLayerScript.
            // Preserve the assigned slot's effect, eye, count and six rects;
            // only rebind its immutable source generation for future frames.
            let opaque = self
                .not_trans
                .get(&picture.slot)
                .copied()
                .unwrap_or(picture.slot < 4 || picture.slot == 14);
            let image = if opaque {
                picture.opaque.clone()
            } else {
                picture.keyed.clone()
            };
            let size = image.size();
            let id = self.layer_id(picture.slot)?;
            let rectangles = self
                .rectangles
                .get(&picture.slot)
                .context("NNN assigned preload rectangle state absent")?
                .clone();
            let published = host.publish_image_resource(self.client, id, image)?;
            self.api_calls += 1;
            if !self.fill_effects.contains_key(&picture.slot) {
                let operation = self.picture_operation(
                    picture.slot,
                    size,
                    published.path.relative(),
                    &rectangles,
                )?;
                if let Err(error) = self.submit(host, json!({"layers":[operation]})) {
                    let _ = host.release_image_resource(self.client, published.path.relative());
                    return Err(error);
                }
            }
            machine.acknowledge(call)?;
            self.images.as_mut().unwrap().accept(call)?;
            if let Some(previous) = self
                .image_paths
                .insert(picture.slot, published.path.relative().to_owned())
            {
                host.release_image_resource(self.client, &previous)?;
            }
        } else {
            machine.acknowledge(call)?;
            self.images.as_mut().unwrap().accept(call)?;
        }
        Ok(true)
    }
    fn picture_operation(
        &self,
        slot: i32,
        size: [u32; 2],
        path: &str,
        rectangles: &crate::native_geometry::NativeRects,
    ) -> Result<Value> {
        let mapping = rectangles.clipped_layer(size)?;
        let empty = mapping.is_none();
        // Keep an empty native copy as an explicitly empty engine source crop,
        // so a later visibility toggle cannot expose a fabricated full image.
        let (rect, uv) = mapping.unwrap_or((
            [
                rectangles.destination[0] as f32,
                rectangles.destination[1] as f32,
                size[0] as f32,
                size[1] as f32,
            ],
            [0., 0., 1., 1.],
        ));
        Ok(
            json!({"op":"put","layer":{"id":self.layer_id(slot)?,"parent":self.bindings.root,"rect":rect,"uv":uv,
                "source_crop":if empty {Some([0f32;4])} else {None},
                "order":[slot,0],"visible":self.layer_visible.get(&slot).copied().unwrap_or(true),"paint":{"kind":"image","asset":path}}}),
        )
    }
    fn restore_picture_source(&mut self, slot: i32, host: &mut dyn HostServices) -> Result<()> {
        if !self.fill_effects.contains_key(&slot) && !self.moves.contains_key(&slot) {
            return Ok(());
        }
        let id = self.layer_id(slot)?;
        if self.picture_assignments.contains(&slot) {
            let picture = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(&slot))
                .ok_or_else(|| anyhow::anyhow!("NNN assigned picture resource absent"))?;
            let size = picture.display.size();
            let path = self
                .image_paths
                .get(&slot)
                .ok_or_else(|| anyhow::anyhow!("NNN assigned picture publication absent"))?;
            let rectangles = self
                .rectangles
                .get(&slot)
                .cloned()
                .unwrap_or_else(|| crate::native_geometry::NativeRects::natural(size));
            let op = self.picture_operation(slot, size, path, &rectangles)?;
            self.submit(host, json!({"layers":[op]}))?;
            self.active_effects.insert(slot);
        } else if self.active_effects.contains(&slot) {
            self.submit(host, json!({"layers":[{"op":"remove","id":id}]}))?;
            self.active_effects.remove(&slot);
        }
        Ok(())
    }
    fn native_picture_frame(
        &self,
        slot: i32,
        count: i32,
        frames: i32,
        command: i32,
    ) -> crate::native_presentation::MoveFrame {
        let picture = self
            .images
            .as_ref()
            .and_then(|images| images.pictures.get(&slot))
            .filter(|_| self.picture_assignments.contains(&slot));
        let opaque = self
            .not_trans
            .get(&slot)
            .copied()
            .unwrap_or(slot < 4 || slot == 14);
        let source = picture.map(|picture| {
            if opaque {
                picture.opaque.clone()
            } else if picture.has_mask {
                picture.raw.clone()
            } else {
                picture.keyed.clone()
            }
        });
        crate::native_presentation::MoveFrame {
            slot,
            state: crate::native_move::NativeMoveState {
                count,
                frames,
                ended: false,
                rectangles: self.rectangles.get(&slot).cloned().unwrap_or_else(|| {
                    crate::native_geometry::NativeRects::natural(self.bindings.size)
                }),
            },
            source,
            math: if opaque {
                crysirror_core::image_processing::ImageCompositeMath::CopyRgb
            } else {
                crysirror_core::image_processing::ImageCompositeMath::MaskSeparate256
            },
            visible: self.layer_visible.get(&slot) != Some(&false),
            native_command: command,
        }
    }
    fn frame_effects(&self) -> BTreeMap<i32, crate::native_presentation::NativeFrameEffect> {
        use crate::native_presentation::NativeFrameEffect;
        let mut effects = BTreeMap::new();
        for (slot, state) in &self.patterns {
            effects.insert(*slot, NativeFrameEffect::Suna(state.clone()));
        }
        for (slot, state) in &self.fades {
            effects.insert(*slot, NativeFrameEffect::Fade(state.clone()));
        }
        for (slot, state) in &self.greys {
            effects.insert(*slot, NativeFrameEffect::Grey(state.clone()));
        }
        for (slot, state) in &self.negas {
            effects.insert(*slot, NativeFrameEffect::Nega(state.clone()));
        }
        for (slot, state) in &self.moves {
            let picture = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(slot))
                .filter(|_| self.picture_assignments.contains(slot));
            let background = self
                .not_trans
                .get(slot)
                .copied()
                .unwrap_or(*slot < 4 || *slot == 14);
            let source = picture.map(|picture| {
                if background {
                    picture.opaque.clone()
                } else if picture.has_mask {
                    picture.raw.clone()
                } else {
                    picture.keyed.clone()
                }
            });
            effects.insert(
                *slot,
                NativeFrameEffect::Move(crate::native_presentation::MoveFrame {
                    slot: *slot,
                    state: state.clone(),
                    source,
                    math: if background {
                        crysirror_core::image_processing::ImageCompositeMath::CopyRgb
                    } else {
                        crysirror_core::image_processing::ImageCompositeMath::MaskSeparate256
                    },
                    visible: self.layer_visible.get(slot) != Some(&false),
                    native_command: 52,
                }),
            );
        }
        for (slot, state) in &self.mosaics {
            effects.insert(*slot, NativeFrameEffect::Mosaic(state.clone()));
        }
        for (slot, state) in &self.picture_effects {
            let background = self.not_trans.get(slot).copied().unwrap_or(*slot < 4 || *slot == 14);
            let uses_mask = state.uses_mask(background);
            let picture = self.images.as_ref().and_then(|images| images.pictures.get(slot))
                .filter(|_| self.picture_assignments.contains(slot));
            let source = picture
                // Actual 4D7380 uses mask helper4D72B0 when present and
                // RGB-key helper4CB1B0 otherwise. The later public branch
                // instead calls mask-only TransLucentBlt3; do not copy it.
                .map(|p| if !uses_mask || (state.command() == 124 && !p.has_mask) { p.opaque.clone() } else if p.has_mask { p.raw.clone() } else { p.keyed.clone() });
            effects.insert(*slot, NativeFrameEffect::Picture(crate::native_picture_effect::PictureEffectFrame {
                slot: *slot, state: state.clone(), source, canvas: self.bindings.size, has_mask: picture.is_some_and(|p| p.has_mask),
                rectangles: self.rectangles.get(slot).cloned().unwrap_or_else(|| crate::native_geometry::NativeRects::natural(self.bindings.size)),
            }));
        }
        for (slot, state) in &self.chara_trans {
            let picture = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(slot))
                .filter(|_| self.picture_assignments.contains(slot));
            let source = picture.map(|p| {
                if p.has_mask {
                    p.raw.clone()
                } else {
                    p.opaque.clone()
                }
            });
            let rectangles = self.rectangles.get(slot).cloned().unwrap_or_else(|| {
                crate::native_geometry::NativeRects::natural(self.bindings.size)
            });
            effects.insert(
                *slot,
                NativeFrameEffect::Chara(crate::native_presentation::CharaTransFrame {
                    state: state.clone(),
                    source,
                    rectangles,
                }),
            );
        }
        for (slot, state) in &self.old_lamps {
            let source = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(slot))
                .filter(|_| {
                    self.picture_assignments.contains(slot)
                        && self.layer_visible.get(slot) != Some(&false)
                })
                .map(|p| p.keyed.clone());
            let r = self.rectangles.get(slot).cloned().unwrap_or_else(|| {
                crate::native_geometry::NativeRects::natural(self.bindings.size)
            });
            effects.insert(
                *slot,
                NativeFrameEffect::Lamp(crate::native_presentation::OldLampFrame {
                    slot: *slot,
                    state: state.clone(),
                    source,
                    destination: [r.destination[0], r.destination[1]],
                }),
            );
        }
        for (slot, state) in &self.buras {
            effects.insert(*slot, NativeFrameEffect::Bura(state.clone()));
        }
        for (slot, state) in &self.scrolls {
            let source = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(slot))
                .filter(|_| self.picture_assignments.contains(slot))
                .map(|picture| picture.opaque.clone());
            effects.insert(
                *slot,
                NativeFrameEffect::Scroll(crate::native_presentation::ScrollLayerFrame {
                    slot: *slot,
                    frame: crate::native_effects_scroll::ScrollFrame {
                        state: state.clone(),
                        canvas: self.bindings.size,
                        source,
                    },
                    normal: self.native_picture_frame(*slot, state.count, state.frames, 0),
                    ended: false,
                }),
            );
        }
        for (slot, state) in &self.flashes {
            effects.insert(
                *slot,
                NativeFrameEffect::Flash(crate::native_effects_flash::FlashFrame {
                    state: state.clone(),
                    canvas: self.bindings.size,
                }),
            );
        }
        for (slot, state) in &self.shakes {
            effects.insert(
                *slot,
                NativeFrameEffect::Shake(crate::native_effects_shake::ShakeFrame {
                    slot: *slot,
                    state: state.clone(),
                    canvas: self.bindings.size,
                    visible: self.layer_visible.get(slot) != Some(&false),
                }),
            );
        }
        for (slot, state) in &self.fusumas {
            let first = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(slot))
                .filter(|_| self.picture_assignments.contains(slot))
                .map(|picture| picture.opaque.clone());
            let second = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(&state.picture_slot()))
                .map(|picture| picture.opaque.clone());
            effects.insert(
                *slot,
                NativeFrameEffect::Fusuma(crate::native_effects_fusuma::FusumaFrame {
                    state: state.clone(),
                    canvas: self.bindings.size,
                    first,
                    second,
                }),
            );
        }
        // CalcuOnly interpolates rectangles for ordinary pictures too. Keep
        // their real command0/count policy instead of freezing rectangles or
        // pretending a From/To pair always means an explicit MOVE52.
        for slot in &self.picture_assignments {
            if !effects.contains_key(slot) && !self.fill_effects.contains_key(slot) {
                let (count, frames) = self.passive_counts.get(slot).copied().unwrap_or((0, 0));
                effects.insert(
                    *slot,
                    NativeFrameEffect::Move(self.native_picture_frame(*slot, count, frames, 0)),
                );
            }
        }
        effects
    }
    fn visible_effects(
        &self,
        effects: &BTreeMap<i32, crate::native_presentation::NativeFrameEffect>,
    ) -> Result<BTreeSet<i32>> {
        use crate::native_presentation::NativeFrameEffect;
        let visible: BTreeSet<_> = effects
            .keys()
            .filter(|slot| self.layer_visible.get(slot) != Some(&false))
            .copied()
            .collect();
        for slot in &visible {
            match &effects[slot] {
                NativeFrameEffect::MessageBackground | NativeFrameEffect::Paint(_) => {
                    bail!("NNN paint DTO in persistent effect state")
                }
                NativeFrameEffect::Suna(state) => {
                    state.validate_curve()?;
                }
                NativeFrameEffect::Fade(state) => {
                    state.validate_curve()?;
                }
                NativeFrameEffect::Grey(state) | NativeFrameEffect::Nega(state) => {
                    state.validate_curve()?;
                }
                NativeFrameEffect::Move(_)
                | NativeFrameEffect::Shake(_)
                | NativeFrameEffect::Flash(_) => {}
                NativeFrameEffect::Scroll(frame) => {
                    frame.frame.state.validate(self.bindings.size)?
                }
                NativeFrameEffect::Fusuma(frame) => frame.state.validate(self.bindings.size)?,
                NativeFrameEffect::Mosaic(_) => {}
                NativeFrameEffect::Picture(frame) => frame.state.validate()?,
                NativeFrameEffect::Chara(_) => {}
                NativeFrameEffect::Lamp(_) => {}
                NativeFrameEffect::Bura(state) => {
                    state.validate(self.bindings.size)?;
                }
            }
        }
        Ok(visible)
    }
    /// Ordinary pictures at/above the first screen effect cannot be flattened
    /// into the lower Canvas. They remain ordered shared image graph inputs.
    fn frame_underlays(
        &self,
        effects: &BTreeMap<i32, crate::native_presentation::NativeFrameEffect>,
        visible: &BTreeSet<i32>,
    ) -> Result<BTreeMap<i32, crysirror_core::image_processing::ImageCompositeLayer>> {
        use crysirror_core::image_processing::{
            ImageCompositeLayer, ImageCompositeMath, ImageInput,
        };
        let mut result = BTreeMap::new();
        let Some(first) = visible.first() else {
            return Ok(result);
        };
        for slot in self.active_effects.range(*first..) {
            if self.layer_visible.get(slot) == Some(&false)
                || self.chara_trans.contains_key(slot)
                || self.old_lamps.contains_key(slot)
                || self.moves.contains_key(slot)
                || matches!(
                    effects.get(slot),
                    Some(
                        NativeFrameEffect::Move(_)
                            | NativeFrameEffect::Scroll(_)
                            | NativeFrameEffect::Fusuma(_)
                            | NativeFrameEffect::Picture(_)
                    )
                )
            {
                continue;
            }
            let layer = if let Some(color) = self.fill_effects.get(slot) {
                let [w, h] = self.bindings.size;
                ImageCompositeLayer {
                    input: Box::new(ImageInput::Solid {
                        size: [w, h],
                        color: [color[0], color[1], color[2], 255],
                    }),
                    source: [0, 0, w as i32, h as i32],
                    destination: [0, 0],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::CopyRgb,
                }
            } else if self.picture_assignments.contains(slot) {
                let picture = self
                    .images
                    .as_ref()
                    .and_then(|i| i.pictures.get(slot))
                    .ok_or_else(|| anyhow::anyhow!("NNN ordered picture resource missing"))?;
                let background = self
                    .not_trans
                    .get(slot)
                    .copied()
                    .unwrap_or(*slot < 4 || *slot == 14);
                let image = if background {
                    picture.opaque.clone()
                } else if picture.has_mask {
                    picture.raw.clone()
                } else {
                    picture.keyed.clone()
                };
                let r =
                    self.rectangles.get(slot).cloned().unwrap_or_else(|| {
                        crate::native_geometry::NativeRects::natural(image.size())
                    });
                ensure!(
                    r.destination_from == r.destination_to,
                    "NNN ordered moving picture needs native frame rectangle interpolation"
                );
                let [x, y, w, h] = r.destination;
                let [sx, sy, _, _] = r.source;
                ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(image)),
                    source: [sx, sy, w, h],
                    destination: [x, y],
                    clip: None,
                    opacity: 255,
                    math: if background {
                        ImageCompositeMath::CopyRgb
                    } else {
                        ImageCompositeMath::MaskSeparate256
                    },
                }
            } else {
                // A native enabled NOP with pic==-1 still owns its counter,
                // but contributes no pixels and has no engine Layer to copy.
                continue;
            };
            result.insert(*slot, layer);
        }
        Ok(result)
    }
    fn prepare_frame_canvas(
        &mut self,
        host: &mut dyn HostServices,
        first: Option<i32>,
    ) -> Result<u32> {
        let [w, h] = self.bindings.size;
        let canvas = self
            .bindings
            .overlap_canvas
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("NNN frame Canvas identifier overflow"))?;
        let mut canvases = vec![if self.frame_canvas_created {
            json!({"op":"restore","id":canvas,"surface":{"width":w,"height":h,"nodes":[]}})
        } else {
            json!({"op":"create","id":canvas,"width":w,"height":h})
        }];
        canvases.push(json!({"op":"clear","id":canvas,"color":[0,0,0,255]}));
        for slot in &self.active_effects {
            if self.layer_visible.get(slot) == Some(&false)
                || first.is_some_and(|first| *slot >= first)
                || self.chara_trans.contains_key(slot)
                || self.old_lamps.contains_key(slot)
                || self.moves.contains_key(slot)
                || (!self.picture_assignments.contains(slot)
                    && !self.fill_effects.contains_key(slot))
            {
                continue;
            }
            canvases.push(json!({"op":"copy_subtree","source":self.layer_id(*slot)?,"destination":canvas,"rect":[0,0,w,h],"position":[0,0]}));
        }
        self.submit(host, json!({"canvases":canvases}))?;
        self.frame_canvas_created = true;
        Ok(canvas)
    }
    fn try_draw(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        use crate::native_presentation::NativeFrameWork;
        let frames = match &call.payload {
            Payload::Words { count: 1, words }
                if words.len() == 1 && (0..=864000).contains(&words[0]) =>
            {
                words[0]
            }
            Payload::Control {
                code: 0x5556000c | 0x5556000e | 0x55560013,
                ..
            } if self.background_read_marks.is_some() && call.identify == 0x66660002 => 1,
            _ => bail!("NNN native DRAW/background-frame payload"),
        };
        if self.draw_wait.is_none()
            && frames > 0
            && self.skip.next_command
            && self.background_read_marks.is_none()
        {
            let mut policy = self.message_control();
            policy.before_print(self.message_window_visible);
            if let Some(text) = self.text.as_mut() {
                ensure!(
                    !text.pending_message(),
                    "NNN skipped DRAW during message wait"
                );
                // Actual4ff0f0 only writes printMode=DRAW and drawCount. No
                // Calcu/CountUp/Print is executed until a real actor resumes.
                text.state.mode = 0x22220004;
                text.state.window_visible = policy.window_visible;
            }
            machine.acknowledge(call)?;
            self.adopt_message_control(policy);
            return Ok(true);
        }
        if self.draw_wait.is_none()
            && frames > 0
            && (!self.frame_admission || self.actor_clock_at == Some(host.now_ms()))
        {
            return Ok(false);
        }
        if self.draw_wait.is_none() {
            let background = self.background_read_marks.is_some();
            let mut timing = crate::native_draw_clock::DrawClock::new(
                self.actor_clock.clone(),
                self.actor_clock_at,
                if background {
                    self.last_background_steps
                } else {
                    self.last_native_steps
                },
                frames.try_into()?,
            )?;
            if background {
                timing = timing.background(self.background_maximum_steps);
            }
            timing.admit(host.now_ms(), !background && self.draw_skip_requested())?;
            let mut policy = self.message_control();
            let visible = policy.before_print(self.message_window_visible);
            let mut view = if self.background_read_marks.is_none() {
                self.text
                    .as_ref()
                    .map(|text| text.draw_view(visible))
                    .transpose()?
            } else {
                None
            };
            if let Some(view) = view.as_mut() {
                if self.post_text_path.is_some() {
                    view.hide
                        .push(self.bindings.effect_base + u32::from(self.bindings.layers) + 1);
                }
            }
            if self.background_read_marks.is_none() {
                if let Some(text) = self.text.as_mut() {
                    text.accept_draw_mode(visible)?;
                }
            }
            self.adopt_message_control(policy);
            let effects = self.frame_effects();
            let visible = self.visible_effects(&effects)?;
            let random = machine
                .native_variables_candidate(call)?
                .random_state()
                .ok_or_else(|| anyhow::anyhow!("NNN DRAW requires original shared random state"))?;
            let underlays = self.frame_underlays(&effects, &visible)?;
            let canvas = self.prepare_frame_canvas(host, visible.first().copied())?;
            let display = self.bindings.effect_base + u32::from(self.bindings.layers);
            ensure!(display != self.bindings.root, "NNN display/root collision");
            let work = NativeFrameWork::begin_draw(
                self.client,
                canvas,
                crate::native_draw::ordered_draw_plan(
                    &effects,
                    &visible,
                    &underlays,
                    view.as_ref(),
                ),
                random,
                frames > 0,
                host,
            )?;
            self.api_calls += work.api_calls;
            let mut draw = crate::native_draw::NativeDraw::begin(
                self.client,
                display,
                self.bindings.layers,
                self.bindings.size,
                self.frame_time_ms,
                call,
                work,
                effects,
                visible,
                underlays,
                self.display_path.take(),
                host.now_ms(),
            )?;
            if let Some(view) = view {
                draw.set_message_view(view)?;
            }
            draw.set_clock(timing)?;
            self.draw_wait = Some(draw);
        }
        let skip = self.background_read_marks.is_none() && self.draw_skip_requested();
        let wait = self.draw_wait.as_mut().unwrap();
        let before = wait.api_calls;
        let result = wait.poll_with_admission(machine, call, host, self.frame_admission, skip);
        self.api_calls += wait.api_calls - before;
        if let Some(result) = result? {
            self.retire_post_text(host)?;
            self.adopt_draw_result(result, frames, host)?;
            if frames > 0 {
                self.bind_completed_foreground_mode(machine, 10)?;
            }
            return Ok(true);
        }
        Ok(false)
    }
    fn adopt_draw_result(
        &mut self,
        result: crate::native_draw::DrawResult,
        frames: i32,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        // FLASH CountIsMax changes the real slot to FILL54. Its retained
        // source Layer must change too, otherwise a later static frame would
        // copy the old assigned picture after the final FLASH frame retires.
        let mut terminal_fills = Vec::new();
        for (slot, effect) in &result.effects {
            if let NativeFrameEffect::Flash(frame) = effect {
                if frame.state.ended {
                    let [red, green, blue] = frame.state.terminal_color();
                    let [w, h] = self.bindings.size;
                    terminal_fills.push(json!({"op":"put","layer":{"id":self.layer_id(*slot)?,
                        "parent":self.bindings.root,"visible":self.layer_visible.get(slot).copied().unwrap_or(true),
                        "rect":[0,0,w,h],"paint":{"kind":"solid","color":[red,green,blue,255]},"order":[slot,0]}}));
                }
            }
        }
        let flash_transition = !terminal_fills.is_empty();
        if flash_transition {
            self.submit(host, json!({"layers":terminal_fills}))?;
        }
        let frames = if let Some(clock) = result.clock.as_ref() {
            i32::try_from(clock.consumed)?
        } else {
            frames
        };
        if let Some(clock) = result.clock.as_ref() {
            self.actor_clock = clock.clock.clone();
            self.actor_clock_at = clock.at;
            if clock.background {
                self.last_background_steps = clock.last_steps;
            } else {
                self.last_native_steps = clock.last_steps;
            }
        }
        if frames > 0 {
            for (count, max) in self.passive_counts.values_mut() {
                if *count < *max {
                    *count = count.saturating_add(frames).min(*max);
                }
            }
        }
        if let Some(pattern) = &result.frame.shared_pattern {
            self.pattern_cache = Some(pattern.clone());
        }
        // Native FADEIN CountIsMax turns into NOP, disabling a picture-less
        // slot; MOVE similarly becomes NOP while retaining its terminal rect.
        // CountUp follows Print, so a waiting actor owes one final repaint.
        let retired_fades: Vec<_> = self
            .fades
            .iter()
            .filter(|(slot, _)| !result.effects.contains_key(slot))
            .map(|(slot, state)| (*slot, state.frames))
            .chain(
                self.shakes
                    .iter()
                    .filter(|(slot, _)| !result.effects.contains_key(slot))
                    .map(|(slot, state)| (*slot, state.frames)),
            )
            .collect();
        self.actor_refresh_required = !retired_fades.is_empty() || flash_transition;
        for (slot, frames) in retired_fades {
            // CountIsMax changes command, never previousEffect (4c18e0
            // only assigns that field after an explicit SetEffectScript).
            if self.picture_assignments.contains(&slot) {
                self.passive_counts.insert(slot, (frames, frames));
            } else {
                self.active_effects.remove(&slot);
                self.passive_counts.remove(&slot);
                self.not_trans.insert(slot, slot < 4 || slot == 14);
            }
        }
        self.patterns.clear();
        self.fades.clear();
        self.greys.clear();
        self.negas.clear();
        self.chara_trans.clear();
        self.old_lamps.clear();
        self.buras.clear();
        self.moves.clear();
        self.scrolls.clear();
        self.fusumas.clear();
        self.shakes.clear();
        self.flashes.clear();
        self.picture_effects.clear();
        self.mosaics.clear();
        for (slot, effect) in result.effects {
            match effect {
                NativeFrameEffect::MessageBackground | NativeFrameEffect::Paint(_) => {
                    bail!("NNN paint DTO returned as semantic state")
                }
                NativeFrameEffect::Move(frame) => {
                    self.rectangles.insert(slot, frame.state.rectangles.clone());
                    if frame.native_command == 0 {
                        self.actor_refresh_required |= frame.state.count >= frame.state.frames
                            && frame.state.rectangles.destination
                                != frame.state.rectangles.destination_to;
                        self.passive_counts
                            .insert(slot, (frame.state.count, frame.state.frames));
                    } else if frame.state.ended {
                        self.actor_refresh_required = true;
                        self.passive_counts
                            .insert(slot, (frame.state.count, frame.state.frames));
                    } else {
                        self.moves.insert(slot, frame.state);
                    }
                }
                NativeFrameEffect::Scroll(frame) => {
                    self.rectangles
                        .insert(slot, frame.normal.state.rectangles.clone());
                    if frame.ended {
                        self.actor_refresh_required = true;
                        self.passive_counts
                            .insert(slot, (frame.frame.state.count, frame.frame.state.frames));
                    } else {
                        self.scrolls.insert(slot, frame.frame.state);
                    }
                }
                NativeFrameEffect::Fusuma(frame) => {
                    self.fusumas.insert(slot, frame.state);
                }
                NativeFrameEffect::Suna(state) => {
                    self.patterns.insert(slot, state);
                }
                NativeFrameEffect::Fade(state) => {
                    self.fades.insert(slot, state);
                }
                NativeFrameEffect::Grey(state) => {
                    self.greys.insert(slot, state);
                }
                NativeFrameEffect::Nega(state) => {
                    self.negas.insert(slot, state);
                }
                NativeFrameEffect::Mosaic(state) => {
                    self.mosaics.insert(slot, state);
                }
                NativeFrameEffect::Picture(frame) => {
                    self.rectangles.insert(slot, frame.rectangles);
                    self.picture_effects.insert(slot, frame.state);
                }
                NativeFrameEffect::Chara(frame) => {
                    self.chara_trans.insert(slot, frame.state);
                }
                NativeFrameEffect::Lamp(frame) => {
                    self.old_lamps.insert(slot, frame.state);
                }
                NativeFrameEffect::Bura(state) => {
                    self.buras.insert(slot, state);
                }
                NativeFrameEffect::Shake(frame) => {
                    self.shakes.insert(slot, frame.state);
                }
                NativeFrameEffect::Flash(frame) => {
                    if frame.state.ended {
                        self.fill_effects.insert(slot, frame.state.terminal_color());
                        self.passive_counts
                            .insert(slot, (frame.state.count, frame.state.frames));
                        self.active_effects.insert(slot);
                    } else {
                        self.flashes.insert(slot, frame.state);
                    }
                }
            }
        }
        self.message_background = result.frame.message_background;
        self.last_frame = Some(result.frame.target);
        self.display_path = result.display_path;
        self.draw_wait = None;
        Ok(())
    }
    /// Explicit outer-mode cancellation preserves completed shared-frame RNG
    /// without pretending the remaining native DRAW/OVERRAP count executed.
    pub(crate) fn poll_outer_exit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        program: &crate::Program,
        host: &mut dyn HostServices,
    ) -> Result<Option<crate::expression::Variables>> {
        ensure!(
            self.background_read_marks.is_some(),
            "NNN outer exit requires a background native context"
        );
        self.poll_native_exit(machine, call, program, host, false)
    }
    pub(crate) fn poll_shutdown(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        program: &crate::Program,
        host: &mut dyn HostServices,
    ) -> Result<Option<crate::expression::Variables>> {
        self.poll_native_exit(machine, call, program, host, true)
    }
    fn poll_native_exit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        program: &crate::Program,
        host: &mut dyn HostServices,
        shutdown: bool,
    ) -> Result<Option<crate::expression::Variables>> {
        if let Some(wait) = self.draw_wait.as_mut() {
            let before = wait.api_calls;
            let result = wait.cancel_at_frame(machine, call, host);
            self.api_calls += wait.api_calls - before;
            let Some(result) = result? else {
                return Ok(None);
            };
            let mut variables = machine.native_variables_candidate(call)?;
            variables.replace_random_state(result.frame.random.clone())?;
            variables.finish_native_instruction();
            self.adopt_draw_result(result, 0, host)?;
            return Ok(Some(variables));
        }
        if let Some(wait) = self.overlap_wait.as_mut() {
            let before = wait.api_calls;
            let result = wait.cancel_at_frame(machine, call, host);
            self.api_calls += wait.api_calls - before;
            let Some(result) = result? else {
                return Ok(None);
            };
            let mut variables = machine.native_variables_candidate(call)?;
            variables.replace_random_state(result.frame.random.clone())?;
            variables.finish_native_instruction();
            self.retire_post_text(host)?;
            self.message_background = None;
            self.last_frame = Some(result.frame.target);
            self.display_path = result.display_path;
            self.overlap_wait = None;
            return Ok(Some(variables));
        }
        if matches!(
            call.payload,
            Payload::Message { .. } | Payload::Selection { .. }
        ) {
            ensure!(
                shutdown,
                "NNN outer exit from a native input dialog requires its original cancellation policy"
            );
            // WM_CLOSE/EndGame quits the original host; it does not choose a
            // dialog result or acknowledge this pending input instruction.
            let mut variables = machine.native_variables_candidate(call)?;
            variables.finish_native_instruction();
            return Ok(Some(variables));
        }
        if !self.try_commit_with_context(machine, call, program, InputFrame::default(), host)? {
            return Ok(None);
        }
        Ok(Some(
            machine
                .variables()
                .ok_or_else(|| anyhow::anyhow!("NNN outer variable bank absent"))?
                .clone(),
        ))
    }
    fn try_overlap(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        use crate::native_presentation::NativeFrameWork;
        if self.overlap_wait.is_none() {
            let Payload::Words { count, words } = &call.payload else {
                bail!("NNN overlap payload")
            };
            ensure!(
                *count >= 2
                    && words.len() >= 2
                    && words[0] == 0
                    && words[1] > 0
                    && words[1] <= 864000,
                "NNN unsupported native overlap type/length"
            );
            let effects = self.frame_effects();
            let visible = self.visible_effects(&effects)?;
            let underlays = self.frame_underlays(&effects, &visible)?;
            let effects =
                crate::native_presentation::ordered_frame_plan(&effects, &visible, &underlays);
            let candidate = machine.native_variables_candidate(call)?;
            let random = candidate.random_state().ok_or_else(|| {
                anyhow::anyhow!("NNN frame requires original initialized global random state")
            })?;
            let canvas = self.prepare_frame_canvas(host, visible.first().copied())?;
            let display = self.bindings.effect_base + u32::from(self.bindings.layers);
            ensure!(display != self.bindings.root, "NNN display/root collision");
            let work = NativeFrameWork::begin(
                self.client,
                self.bindings.overlap_canvas,
                canvas,
                self.overlap_frame.clone(),
                effects,
                random,
                host,
            )?;
            self.api_calls += work.api_calls;
            self.overlap_wait = Some(crate::native_overlap::NativeOverlap::begin(
                self.client,
                display,
                self.bindings.layers,
                self.bindings.size,
                self.frame_time_ms,
                call,
                work,
                self.display_path.take(),
                host.now_ms(),
            )?);
        }
        let wait = self.overlap_wait.as_mut().unwrap();
        let before = wait.api_calls;
        let result = wait.poll(machine, call, host);
        self.api_calls += wait.api_calls - before;
        if let Some(result) = result? {
            if let Some(pattern) = result.frame.shared_pattern {
                self.pattern_cache = Some(pattern.clone());
                for state in self.patterns.values_mut() {
                    state.resource = pattern.clone();
                }
            }
            self.retire_post_text(host)?;
            self.message_background = None;
            self.last_frame = Some(result.frame.target);
            self.display_path = result.display_path;
            self.overlap_wait = None;
            self.bind_completed_foreground_mode(machine, 14)?;
            return Ok(true);
        }
        Ok(false)
    }
    fn layer_id(&self, slot: i32) -> Result<u32> {
        ensure!(
            (0..i32::from(self.bindings.layers)).contains(&slot),
            "System-NNN effect layer bounds"
        );
        Ok(self.bindings.effect_base + slot as u32)
    }
    fn submit(&mut self, host: &mut dyn HostServices, batch: Value) -> Result<()> {
        host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)?;
        self.api_calls += 1;
        Ok(())
    }
    /// Only a real successful owner operation consumes its exact native receipt.
    /// Unknown/preflight failures retain the pending PC. Original services may
    /// have explicit attempted-write RAM policy (SystemFile does); failure never
    /// fabricates a successful VM acknowledgement.
    pub fn commit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        machine.validate_receipt(call)?;
        if self.skip.next_command
            && self.skip.effect_command
            && ((call.identify == 0x66660007
                && matches!(call.code, 0x33330011 | 0x33330015 | 0x3333001f))
                || (call.identify == 0x66660006
                    && call.code == 0x22220005
                    && self.background_read_marks.is_none()))
        {
            // Actual538b20/541020/540f20 suppress resource/capture work before
            // touching its parameters. Actual53fdc0's back-reader branch is
            // intentionally excluded: it precedes the foreground skip test.
            machine.acknowledge(call)?;
            return Ok(());
        }
        if call.identify == 0x66660007
            && call.code == 0x33330013
            && self.skip.next_command
            && self.skip.effect_command
        {
            return machine.acknowledge(call);
        }
        if matches!(call.payload, Payload::StoryName(_) | Payload::FilmName(_)) {
            return self.metadata.commit(machine, call);
        }
        if let Payload::VariableWrites { writes } = &call.payload {
            let ops=writes.iter().filter(|write|self.active_effects.contains(&i32::from(write.layer)))
                .map(|write|Ok(json!({"op":"set_visible","id":self.layer_id(i32::from(write.layer))?,"visible":write.value==0})))
                .collect::<Result<Vec<_>>>()?;
            if !ops.is_empty() {
                self.submit(host, json!({"layers":ops}))?;
            }
            for write in writes {
                self.layer_visible
                    .insert(i32::from(write.layer), write.value == 0);
                let bit = 1u32 << u32::from(write.layer);
                if write.value == 0 {
                    self.native_layer_off &= !bit;
                } else {
                    self.native_layer_off |= bit;
                }
            }
            return machine.acknowledge(call);
        }
        if let Payload::UserCall { command: false, .. } = &call.payload {
            ensure!(
                machine
                    .native_variables_candidate(call)?
                    .native_writes()
                    .is_empty(),
                "NNN native function arguments with resource-write bindings need an atomic native owner plan"
            );
            let name = self.user_function_name(call)?.to_vec();
            if name.eq_ignore_ascii_case(b"setresult") || name.eq_ignore_ascii_case(b"getresult") {
                let Payload::UserCall { parameters, .. } = &call.payload else {
                    unreachable!()
                };
                let mut variables = machine.native_variables_candidate(call)?;
                if let Some(data) = self.result_data.as_mut() {
                    if data.apply(&name, parameters, &mut variables)? {
                        return machine.acknowledge_with_variables(call, variables);
                    }
                }
            }
            if name.eq_ignore_ascii_case(b"stopf5") {
                machine.acknowledge(call)?;
                self.skip.stop_f5();
                return Ok(());
            }
            if name.eq_ignore_ascii_case(b"layer") {
                let Payload::UserCall { parameters, .. } = &call.payload else {
                    unreachable!()
                };
                if let Some(&slot) = parameters.first() {
                    ensure!(
                        (0..32).contains(&slot),
                        "NNN layer function shift outside defined native profile"
                    );
                    let visible = parameters.get(1).copied().unwrap_or(1) != 0;
                    if slot < i32::from(self.bindings.layers) {
                        let id = self.layer_id(slot)?;
                        let layer: Value = serde_json::from_slice(&host.query_engine(
                            self.client,
                            crysirror_core::service::EngineQuery::Layer(id),
                        )?)?;
                        self.api_calls += 1;
                        if !layer.is_null() {
                            self.submit(
                                host,
                                json!({"layers":[{"op":"set_visible","id":id,"visible":visible}]}),
                            )?;
                        }
                        self.layer_visible.insert(slot, visible);
                    }
                    let bit = 1u32 << slot as u32;
                    if visible {
                        self.native_layer_off &= !bit;
                    } else {
                        self.native_layer_off |= bit;
                    }
                }
                return machine.acknowledge(call);
            }
            bail!(
                "unsupported System-NNN native function pc={:#x} index={} name={}; exact pending call retained",
                call.pc,
                call.code,
                String::from_utf8_lossy(&name)
            );
        }
        ensure!(
            call.identify == 0x66660007,
            "System-NNN callback needs another semantic mapping"
        );
        match (self.profile, call.code, &call.payload) {
            (_, 0x33330028, Payload::Words { count: 1, words }) if words.len() == 1 => {
                // Actual530c70 branches >0 / ==0 / <0. SetFrameRate's
                // parameter is the original millisecond period, not FPS:
                // it writes frameTime and CShakin.SetFrame preserves debt.
                if words[0] > 0 {
                    self.set_frame_time(words[0] as u64)?;
                } else if words[0] == 0 {
                    self.set_frame_time(self.default_frame_time_ms)?;
                }
            }
            (_, 0x33330029, Payload::Empty) => {
                // Actual530c50 calls the same default-period virtual644.
                self.set_frame_time(self.default_frame_time_ms)?;
            }
            (_, 0x33330019, Payload::Words { count: 1, words }) if words.len() == 1 => {
                // Actual541af0 retired SETSCENE only returns its unused data
                // pointer. It has no SceneFlag write or native mode change.
            }
            // Original legacy ReceiveScriptCommand 0x541af0 has no branch for
            // this code; original ExecScript 0x4ea210 passes its scalar then advances.
            (CallbackProfile::Mugen2Legacy, 0x33330052, Payload::Scalar(_)) => {}
            (_, 0x33330018, Payload::Words { count, words }) => {
                ensure!(
                    *count as usize == words.len() && words.len() >= 2,
                    "NNN SETCG native parameter span"
                );
                let mut allowed = self.background_read_marks != Some(false)
                    && !(self.skip.next_command && !self.must_set_cg);
                if self.off_layer_dont_set_cg {
                    let layer = words.get(2).copied().unwrap_or(-1);
                    if layer >= 0 {
                        // Actual532560 initializes mask0 for layer0; positive
                        // counts use x86's masked SHL count. Preserve the old
                        // module's policy rather than later source's bit1.
                        let mask = if layer == 0 {
                            0
                        } else {
                            1u32 << ((layer as u32) & 31)
                        };
                        if self.native_layer_off & mask != 0 {
                            allowed = false;
                        }
                    }
                }
                if allowed {
                    self.system_file
                        .as_mut()
                        .context("NNN SETCG original persistent flags absent")?
                        .mark_cg(words[0].wrapping_sub(1), words[1].wrapping_sub(1))?;
                }
            }
            (CallbackProfile::Mugen2Legacy, 0x3333001a, Payload::Words { count, words }) => {
                ensure!(
                    *count >= 1 && *count as usize == words.len(),
                    "NNN SETFILM native parameter span"
                );
                // Original0x5324b0: backscript policy and skipped-command
                // policy independently suppress the mark. No I/O occurs here;
                // the in-memory original record is saved at native boundaries.
                if self.background_read_marks != Some(false)
                    && !(self.skip.next_command && !self.must_set_film)
                {
                    let script = self
                        .system_script
                        .context("NNN SETFILM original project index absent")?;
                    self.system_file
                        .as_mut()
                        .context("NNN SETFILM original persistent SystemFile not prepared")?
                        .mark_film(script, words[0].wrapping_sub(1))?;
                }
            }
            (_, 0x33330012, Payload::Empty) => self.clear_all_native_effects(host)?,
            (_, 0x33330011, Payload::Empty) => {
                let [w, h] = self.bindings.size;
                let mut ops = vec![if self.overlap_captures == 0 {
                    json!({"op":"create","id":self.bindings.overlap_canvas,"width":w,"height":h})
                } else {
                    json!({"op":"restore","id":self.bindings.overlap_canvas,"surface":{"width":w,"height":h,"nodes":[]}})
                }];
                ops.push(if let Some(path)=&self.display_path{
                    json!({"op":"set","id":self.bindings.overlap_canvas,"layer":{"id":1,"rect":[0,0,w,h],"paint":{"kind":"image","asset":path}}})
                }else{json!({"op":"copy_subtree","source":self.bindings.root,"destination":self.bindings.overlap_canvas,"rect":[0,0,w,h],"position":[0,0]})});
                self.submit(host, json!({"canvases":ops}))?;
                self.overlap_captures += 1;
                self.overlap_frame = self.last_frame.clone();
            }
            (_, code @ (0x3333001b | 0x3333001c), Payload::Words { count, words }) => {
                ensure!(
                    *count as usize == words.len(),
                    "NNN original message control parameter span"
                );
                let mut policy = self.message_control();
                let mut skip = self.skip.clone();
                let needs_history = skip.next_command
                    && skip.f4
                    && (code == 0x3333001b || skip.effect_command)
                    && words.first().is_some_and(|id| *id != -1);
                let read = if needs_history {
                    Some(
                        self.system_file
                            .as_ref()
                            .context("NNN message-control original SystemFile absent")?
                            .message(
                                self.system_script
                                    .context("NNN message-control source-reader index absent")?,
                                words[0],
                            )?,
                    )
                } else {
                    None
                };
                if code == 0x3333001b {
                    policy.start_message_with_history(words, &mut skip, read)?;
                } else {
                    policy.start_koma_with_history(words, &mut skip, read)?;
                }
                machine.acknowledge(call)?;
                self.adopt_message_control(policy);
                self.skip = skip;
                return Ok(());
            }
            (_, 0x33330007, Payload::Words { count: 1, words }) if words == &[-1] => {
                // No source has been admitted by this bounded bridge. A live
                // source needs original fade-unit/settings policy, not a guessed0ms stop.
                ensure!(
                    !host.audio_playing(self.client, self.bindings.music_channel)?,
                    "System-NNN active music needs native fade/settings mapping"
                );
                host.submit_audio(
                    self.client,
                    AudioRequest::Stop {
                        channel: self.bindings.music_channel,
                        fade_ms: 0.,
                    },
                )?;
                self.api_calls += 1;
            }
            (CallbackProfile::Mugen2Legacy, 0x33330052, Payload::Scalar(_)) => {
                // FILMTYPETIME is a known scalar SPT tag, but complete legacy
                //541af0 has no dispatch or helper for52 (50/51 then53). The
                // original void callback discards it; no timeout or success
                // substitute is invented for another/native unknown command.
            }
            (_, 0x33330017, Payload::Empty) => {
                //541af0 writes252=TRUE despite this tag's misleading name.
                // The real DRAW window painter consumes it before layer15.
                ensure!(
                    self.text.is_some(),
                    "NNN NOMESSAGEWINDOW needs native window capability"
                );
                self.message_window_visible = true;
            }
            (_, 0x3333001e, Payload::Scalar(value)) => {
                // Actual541af0 maps2->0;52f710 stores the native flag and
                // calls SetSystemParam(6). Current message actors read that
                // SystemFile preference on every admitted update, including
                // retained waits; this is not the separate AutoMessage speed.
                let value = if *value == 2 { 0 } else { *value };
                let mut file = self
                    .system_file
                    .clone()
                    .context("NNN SETDEMOFLAG original SystemFile absent")?;
                file.set_auto_mode_raw(value)?;
                machine.acknowledge(call)?;
                self.system_file = Some(file);
                self.legacy_demo_flag = value;
                return Ok(());
            }
            (_, 0x3333005a, Payload::Scalar(value)) => {
                self.option_off = if *value == -1 { 0 } else { *value };
            }
            (CallbackProfile::Mugen2Legacy, 0x33330053, Payload::Scalar(value)) => {
                //541af0/401ea0: zero leaves the current mask unchanged.
                if *value > 0 {
                    self.configuration_mask = *value;
                } else if *value < 0 {
                    self.configuration_mask = 0;
                }
            }
            (CallbackProfile::Mugen2Legacy, 0x33330055, Payload::Scalar(value)) => {
                // Actual52eee0 only writes CGame+7328*4. All original code
                // references are this setter and53ad20's STATUS182 producer;
                // no cursor/device action exists in this legacy profile.
                //53a710 does not reset it when starting a new game.
                self.script_mouse_number = *value;
            }
            (CallbackProfile::Mugen2Legacy, 0x33330056, Payload::Scalar(value)) => {
                //52eea0/53ad20: raw script number, preserved as STATUS181.
                self.script_cursor_number = *value;
            }
            (CallbackProfile::Mugen2Legacy, 0x3333005c, Payload::Scalar(value)) => {
                //530BE0 calls4FE990 only if the message actor exists. Only1
                //sets the latch;0/-1 do not clear it. This field is neither
                //read nor serialized by the complete reviewed legacy build.
                if *value == 1 && self.text.is_some() {
                    self.obsolete_message_exp_wait = true;
                }
            }
            (CallbackProfile::Mugen2Legacy, 0x3333005e, Payload::Scalar(_)) => {
                //530b90 dispatches vtable+1020. Actual CGame VT564ad4 binds
                //401650, an empty method. This proven legacy hook has no
                //effect; other command/profile gaps must still fail visibly.
            }
            (_, code @ (0x33330057 | 0x33330058 | 0x33330059), Payload::Scalar(value)) => {
                let value = if *value == -1 { 0 } else { *value };
                match code {
                    0x33330057 => {
                        ensure!(
                            value <= 5,
                            "NNN automatic message speed outside native five-entry table"
                        );
                        self.auto_message = value;
                        if value > 0 && self.auto_kill_skip {
                            self.message_skip = false;
                        }
                    }
                    0x33330058 => self.cannot_click = Some(value != 0),
                    0x33330059 => {
                        self.cannot_skip = value;
                        if value > 0 && self.cannot_kill_skip {
                            self.message_skip = false;
                        }
                    }
                    _ => unreachable!(),
                }
            }
            (_, 0x33330016, Payload::Words { count, words }) => {
                ensure!(
                    *count as usize == words.len() && words.len() >= 9,
                    "NNN SETEFFECTRECT requires slot and eight original coordinates"
                );
                let slot = words[0];
                self.layer_id(slot)?;
                let mut rectangles = self.rectangles.get(&slot).cloned().unwrap_or_else(|| {
                    crate::native_geometry::NativeRects::natural(self.bindings.size)
                });
                rectangles.set_destination_pair(
                    words[1..5].try_into().unwrap(),
                    words[5..9].try_into().unwrap(),
                );
                if self.picture_assignments.contains(&slot)
                    && !self.fill_effects.contains_key(&slot)
                {
                    let picture = self
                        .images
                        .as_ref()
                        .and_then(|i| i.pictures.get(&slot))
                        .ok_or_else(|| anyhow::anyhow!("NNN assigned geometry picture missing"))?;
                    let path = self.image_paths.get(&slot).ok_or_else(|| {
                        anyhow::anyhow!("NNN assigned geometry publication missing")
                    })?;
                    let op =
                        self.picture_operation(slot, picture.display.size(), path, &rectangles)?;
                    self.submit(host, json!({"layers":[op]}))?;
                }
                machine.acknowledge(call)?;
                if let Some(state) = self.moves.get_mut(&slot) {
                    state.rectangles = rectangles.clone();
                }
                self.rectangles.insert(slot, rectangles);
                return Ok(());
            }
            (_, code @ (0x33330014 | 0x33330013), Payload::Words { words, .. })
                if code == 0x33330014 && words.len() == 1
                    || code == 0x33330013 && words.len() >= 2 && words[1] == -1 =>
            {
                let id = self.layer_id(words[0])?;
                let ops = if self.active_effects.contains(&words[0]) {
                    vec![json!({"op":"remove","id":id})]
                } else {
                    vec![]
                };
                self.submit(host, json!({"layers":ops}))?;
                self.active_effects.remove(&words[0]);
                self.inactive_effect_records.remove(&words[0]);
                self.patterns.remove(&words[0]);
                self.fades.remove(&words[0]);
                self.greys.remove(&words[0]);
                self.negas.remove(&words[0]);
                self.chara_trans.remove(&words[0]);
                self.old_lamps.remove(&words[0]);
                self.buras.remove(&words[0]);
                self.moves.remove(&words[0]);
                self.scrolls.remove(&words[0]);
                self.fusumas.remove(&words[0]);
                self.shakes.remove(&words[0]);
                self.flashes.remove(&words[0]);
                self.picture_effects.remove(&words[0]);
                self.mosaics.remove(&words[0]);
                self.fill_effects.remove(&words[0]);
                self.passive_counts.remove(&words[0]);
                self.picture_assignments.remove(&words[0]);
                self.rectangles.remove(&words[0]);
                self.not_trans
                    .insert(words[0], words[0] < 4 || words[0] == 14);
                self.actor_refresh_required = true;
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && matches!(words[1], 8 | 43 | 90 | 99 | 124) => {
                ensure!(*count as usize == words.len(), "NNN picture-effect parameter span");
                let slot = words[0];
                self.layer_id(slot)?;
                let mosaic = if words[1] == 43 { Some(crate::native_effects_mosaic::MosaicState::parse(&words[2..], self.bindings.size)?) } else { None };
                let state = if words[1] != 43 { Some(crate::native_picture_effect::PictureEffectState::parse(words[1], &words[2..])?) } else { None };
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.passive_counts.remove(&slot);
                self.active_effects.insert(slot);
                self.previous_effects.insert(slot, words[1]);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                if let Some(state) = state { self.picture_effects.insert(slot, state); }
                if let Some(state) = mosaic { self.mosaics.insert(slot, state); }
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 52 => {
                ensure!(
                    *count as usize == words.len() && words.len() <= 258,
                    "NNN Move parameter span"
                );
                let slot = words[0];
                self.layer_id(slot)?;
                let rectangles = self.rectangles.get(&slot).cloned().unwrap_or_else(|| {
                    crate::native_geometry::NativeRects::natural(self.bindings.size)
                });
                let state = crate::native_move::NativeMoveState::parse(&words[2..], rectangles)?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.passive_counts.remove(&slot);
                self.active_effects.insert(slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.moves.insert(slot, state);
                self.previous_effects.insert(slot, 52);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 54 => {
                let slot = words[0];
                let id = self.layer_id(slot)?;
                // EFFECT_FILL's first parameter is reserved. Absent RGB uses0;
                // green inherits red, blue inherits green; stored bytes wrap.
                let red = words.get(3).copied().unwrap_or(0) as u8;
                let green = words.get(4).copied().unwrap_or(i32::from(red)) as u8;
                let blue = words.get(5).copied().unwrap_or(i32::from(green)) as u8;
                let [w, h] = self.bindings.size;
                self.submit(host,json!({"layers":[{"op":"put","layer":{"id":id,"parent":self.bindings.root,"visible":self.layer_visible.get(&slot).copied().unwrap_or(true),"rect":[0,0,w,h],"paint":{"kind":"solid","color":[red,green,blue,255]},"order":[slot,0]}}]}))?;
                self.active_effects.insert(slot);
                self.previous_effects.insert(slot, 54);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.fill_effects.insert(slot, [red, green, blue]);
                self.passive_counts.insert(slot, (0, 9999));
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 1 => {
                ensure!(
                    *count as usize == words.len() && words.len() <= 258,
                    "MAE native parameter span"
                );
                self.layer_id(words[0])?;
                // Reviewed legacy SetEffectScript0x4c18e0 returns immediately
                // for native1: no SetParam, count reset or command replacement.
                // This is the explicit keep-current operation, not an unknown ACK.
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 74 => {
                ensure!(*count as usize == words.len(), "BURA payload count");
                let slot = words[0];
                self.layer_id(slot)?;
                let state = crate::native_bura::BuraState::parse(&words[2..], self.bindings.size)?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.passive_counts.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.buras.insert(slot, state);
                self.previous_effects.insert(slot, 74);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 109 => {
                ensure!(*count as usize == words.len(), "OLDLAMP payload count");
                let slot = words[0];
                self.layer_id(slot)?;
                let state = crate::native_old_lamp::OldLampState::parse(&words[2..])?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.passive_counts.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.old_lamps.insert(slot, state);
                self.previous_effects.insert(slot, 109);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 9 => {
                ensure!(*count as usize == words.len(), "CTRANS payload count");
                let slot = words[0];
                self.layer_id(slot)?;
                let state = crate::native_chara_trans::CharaTransState::parse(
                    &words[2..],
                    self.bindings.size,
                )?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.passive_counts.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.chara_trans.insert(slot, state);
                self.previous_effects.insert(slot, 9);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 17 => {
                ensure!(*count as usize == words.len(), "NNN FLASH parameter count");
                let slot = words[0];
                self.layer_id(slot)?;
                let state = crate::native_effects_flash::FlashState::parse(&words[2..]);
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.passive_counts.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.insert(slot, state);
                self.previous_effects.insert(slot, 17);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 18 => {
                ensure!(*count as usize == words.len(), "NNN SHAKE parameter count");
                let slot = words[0];
                self.layer_id(slot)?;
                let state = crate::native_effects_shake::ShakeState::parse(&words[2..]);
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.passive_counts.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.shakes.insert(slot, state);
                self.previous_effects.insert(slot, 18);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 39 => {
                let slot = words[0];
                self.layer_id(slot)?;
                let state = crate::native_effects_scroll::ScrollState::parse(
                    &words[2..],
                    self.bindings.size,
                )?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.passive_counts.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.scrolls.insert(slot, state);
                self.previous_effects.insert(slot, 39);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 84 => {
                let slot = words[0];
                self.layer_id(slot)?;
                let state = crate::native_effects_fusuma::FusumaState::parse(
                    &words[2..],
                    self.bindings.size,
                )?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.passive_counts.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.fusumas.insert(slot, state);
                self.previous_effects.insert(slot, 84);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 31 => {
                let slot = words[0];
                self.layer_id(slot)?;
                let grey =
                    crate::native_effects::LayerGreyState::parse(&words[2..words.len().min(7)])?;
                grey.validate_curve()?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.passive_counts.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.greys.remove(&slot);
                self.negas.insert(slot, grey);
                self.previous_effects.insert(slot, 31);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 129 => {
                let slot = words[0];
                self.layer_id(slot)?;
                let grey = crate::native_effects::LayerGreyState::parse(&words[2..])?;
                grey.validate_curve()?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.passive_counts.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.negas.remove(&slot);
                self.greys.insert(slot, grey);
                self.previous_effects.insert(slot, 129);
            }
            (_, 0x33330013, Payload::Words { count, words })
                if *count >= 2 && matches!(words[1], 10 | 11) =>
            {
                let slot = words[0];
                self.layer_id(slot)?;
                let fade = if words[1] == 11 {
                    FadeInState::parse_out(&words[2..])?
                } else {
                    FadeInState::parse(&words[2..])?
                };
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.passive_counts.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.fades.insert(slot, fade);
                self.previous_effects.insert(slot, words[1]);
            }
            (_, 0x33330013, Payload::Words { count, words }) if *count >= 2 && words[1] == 0 => {
                let slot = words[0];
                self.layer_id(slot)?;
                self.restore_picture_source(slot, host)?;
                self.fill_effects.remove(&slot);
                self.patterns.remove(&slot);
                self.fades.remove(&slot);
                self.greys.remove(&slot);
                self.negas.remove(&slot);
                self.chara_trans.remove(&slot);
                self.old_lamps.remove(&slot);
                self.buras.remove(&slot);
                self.moves.remove(&slot);
                self.scrolls.remove(&slot);
                self.fusumas.remove(&slot);
                self.shakes.remove(&slot);
                self.flashes.remove(&slot);
                self.picture_effects.remove(&slot);
                self.mosaics.remove(&slot);
                self.passive_counts.insert(slot, (0, 99999));
                self.previous_effects.insert(slot, 0);
            }
            _ => bail!(
                "unsupported System-NNN native pc={:#x} code={:#x}; exact pending call retained",
                call.pc,
                call.code
            ),
        }
        machine.acknowledge(call)
    }
}

#[cfg(test)]
mod system_file_tests {
    use super::*;
    use crate::native_system_file::{NativeSystemFile, SystemLimits, SystemNames, SystemStamp};
    #[test]
    fn original_setfilm_marks_only_current_script_and_respects_both_suppression_gates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project: crysirror_runtime::Project = serde_json::from_value(json!({"schema":2,
            "name":"native persistent flags", "content_revision":"1", "assets":"game", "canvas":[16,16],
            "clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        for (background, skipping, must, expected) in [
            (None, false, false, true),
            (Some(false), false, true, false),
            (Some(true), false, true, true),
            (None, true, false, false),
            (None, true, true, true),
            (Some(false), true, true, false),
        ] {
            let mut words = vec![0u32; 32];
            words[0] = 32;
            words[1] = 0x66660001;
            words[2] = 0x55550001;
            words.extend([5, 0x66660007, 0x3333001a, 1, 2]);
            words[18] = 37;
            let bytes: Vec<_> = words
                .into_iter()
                .flat_map(|word| (!word).to_le_bytes())
                .collect();
            let program = crate::Program::parse(&bytes, crate::XorProfile::default()).unwrap();
            let mut machine = Machine::default();
            let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
                panic!()
            };
            let mut bridge = Bridge::new(
                ClientId(1),
                CallbackProfile::Mugen2Legacy,
                SceneBindings {
                    root: 1,
                    overlap_canvas: 7,
                    effect_base: 100,
                    size: [16, 16],
                    layers: 16,
                    music_channel: 0,
                },
            )
            .unwrap();
            let defaults = NativeSystemFile::defaults(
                SystemLimits::new(64, 20, 20),
                SystemStamp([2026, 11, 1, 0, 0, 0, 0, 0]),
                &SystemNames {
                    font: b"fixture".to_vec(),
                    surname: vec![],
                    given_name: vec![],
                },
            )
            .unwrap();
            bridge
                .configure_system_file(
                    defaults,
                    &crate::config::Names::parse(
                        format!("mustSetFilmFlag {}", i32::from(must)).as_bytes(),
                        false,
                    )
                    .unwrap(),
                )
                .unwrap();
            bridge.set_system_script(Some(3));
            bridge.skip.next_command = skipping;
            if let Some(allow) = background {
                bridge.configure_background_read_marks(allow).unwrap();
            }
            bridge
                .commit(&mut machine, &call, &mut runtime.host)
                .unwrap();
            let file = bridge.system_file.as_ref().unwrap();
            assert_eq!(file.film(3, 1).unwrap(), expected);
            assert!(!file.film(0, 1).unwrap());
            assert!(!file.film(3, 2).unwrap());
            assert_eq!(file.dirty(), expected);
            assert_eq!(machine.pc(), call.next);
            assert_eq!(bridge.api_calls, 0);
        }
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
        assert_eq!(runtime.host.now_ms(), 0);
    }
}

#[cfg(test)]
mod shutdown_input_tests {
    use super::*;
    #[test]
    fn host_quit_cancels_known_text_wait_without_dialog_result_pc_ack_or_engine_mutation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(json!({"schema":2,"name":"native quit wait",
            "content_revision":"1","assets":"game","canvas":[16,16],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([6, 0x66660006, 0x22220001, 0, 7, 0]);
        words[4] = 1;
        words[5] = 38;
        words.push(39);
        words[18] = 40;
        words.push(u32::from_le_bytes(*b"abc\0"));
        let bytes: Vec<_> = words
            .into_iter()
            .flat_map(|word| (!word).to_le_bytes())
            .collect();
        let program = crate::Program::parse(&bytes, crate::XorProfile::default()).unwrap();
        let mut variables =
            crate::expression::Variables::fresh(crate::expression::VariableProfile::Legacy1000);
        variables.write_stored(205, 55);
        let mut machine = Machine::with_variables(variables);
        let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
            panic!()
        };
        let mut bridge = Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [16, 16],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        bridge.configure_background_read_marks(false).unwrap();
        assert!(
            bridge
                .poll_outer_exit(&mut machine, &call, &program, &mut runtime.host)
                .is_err(),
            "ordinary NewGame must keep its separate native input cancellation boundary"
        );
        bridge.enter_foreground_context();
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        let captured = bridge
            .poll_shutdown(&mut machine, &call, &program, &mut runtime.host)
            .unwrap()
            .unwrap();
        assert_eq!(captured.read_stored(205), 55);
        assert_eq!(machine.pc(), call.pc);
        machine.validate_receipt(&call).unwrap();
        assert_eq!(bridge.api_calls, 0);
        assert_eq!(runtime.host.now_ms(), 0);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
    }
}

#[cfg(test)]
mod foreground_control_tests {
    use super::*;
    #[test]
    fn foreground_enfilm_resumes_without_background_frame_and_only_resets_active_film_skip() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(json!({"schema":2,"name":"original Yoyaku controls",
            "content_revision":"1","assets":"game","canvas":[16,16],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        for (film_skip, replay) in [(false, false), (true, false), (true, true)] {
            let mut words = vec![0u32; 32];
            words[0] = 32;
            words[1] = 0x66660001;
            words[2] = 0x55550001;
            words.extend([3, 0x66660002, 0x5556000c]);
            words[18] = 35;
            let program = crate::Program::parse(
                &words
                    .into_iter()
                    .flat_map(|word| (!word).to_le_bytes())
                    .collect::<Vec<_>>(),
                crate::XorProfile::default(),
            )
            .unwrap();
            let mut machine = Machine::default();
            let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
                panic!()
            };
            let mut bridge = Bridge::new(
                ClientId(1),
                CallbackProfile::Mugen2Legacy,
                SceneBindings {
                    root: 1,
                    overlap_canvas: 7,
                    effect_base: 100,
                    size: [16, 16],
                    layers: 16,
                    music_channel: 0,
                },
            )
            .unwrap();
            bridge.skip = crate::native_skip::NativeSkipPolicy {
                f4: true,
                f5: true,
                next_command: true,
                effect_command: true,
                film_end: film_skip,
            };
            bridge.skip_film_enabled = true;
            bridge.scene_replay = replay;
            if replay {
                assert!(
                    bridge
                        .try_commit(&mut machine, &call, &mut runtime.host)
                        .unwrap_err()
                        .to_string()
                        .contains("scene replay")
                );
                assert_eq!(machine.pc(), call.pc);
                machine.validate_receipt(&call).unwrap();
                assert!(bridge.skip_film_enabled);
            } else {
                assert!(
                    bridge
                        .try_commit(&mut machine, &call, &mut runtime.host)
                        .unwrap()
                );
                assert_eq!(machine.pc(), call.next);
                assert!(bridge.skip.f4 && bridge.skip.f5);
                assert_eq!(bridge.skip.next_command, !film_skip);
                assert_eq!(bridge.skip.effect_command, !film_skip);
                assert_eq!(bridge.skip_film_enabled, !film_skip);
                assert!(!bridge.skip.film_end);
            }
            assert_eq!(bridge.api_calls, 0);
            assert!(bridge.draw_wait.is_none());
        }
        assert_eq!(runtime.host.now_ms(), 0);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
    }
}

#[cfg(test)]
mod builtin_function_tests {
    use super::*;
    use crate::{
        config::Names,
        expression::{VariableProfile, Variables},
        native_system_file::{NativeSystemFile, SystemLimits, SystemNames, SystemStamp},
    };
    fn runtime() -> (tempfile::TempDir, crysirror_runtime::Runtime) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project: crysirror_runtime::Project =
            serde_json::from_value(json!({"schema":2,"name":"native function services",
            "content_revision":"1","assets":"game","canvas":[16,16],"clients":[
                {"name":"native","adapter":"crysirror-script-v1","game":"authored"},
                {"name":"other","adapter":"crysirror-script-v1","game":"authored-other"}]}))
            .unwrap();
        let runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        (dir, runtime)
    }
    fn bridge() -> Bridge {
        let mut bridge = Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [16, 16],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        bridge
            .configure_functions(
                Names::parse(b"savesystemfile Native\nlayer Native\nstopf5 Native", false).unwrap(),
            )
            .unwrap();
        bridge
    }
    fn program(record: &[u32]) -> crate::Program {
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend(record);
        words[18] = words.len() as u32;
        crate::Program::parse(
            &words
                .into_iter()
                .flat_map(|word| (!word).to_le_bytes())
                .collect::<Vec<_>>(),
            crate::XorProfile::default(),
        )
        .unwrap()
    }
    fn file() -> NativeSystemFile {
        NativeSystemFile::defaults(
            SystemLimits::new(64, 20, 20),
            SystemStamp([2026, 11, 1, 0, 0, 0, 0, 0]),
            &SystemNames {
                font: b"fixture".to_vec(),
                surname: vec![],
                given_name: vec![],
            },
        )
        .unwrap()
    }
    #[test]
    fn complete_legacy_preference_variable_groups_reach_the_real_vm_worker_from_original_bytes() {
        use crysirror_core::vm::{VmAdapter, VmYield};
        let (_dir, mut runtime) = runtime();
        let mut bridge = bridge();
        let defaults = file();
        let mut bytes = defaults.encode().unwrap().to_vec();
        for (offset, base) in [(1792, -70i32), (1856, 120i32)] {
            for index in 0..16usize {
                let at = 64 + offset + 4 * index;
                bytes[at..at + 4].copy_from_slice(&(base + index as i32).to_le_bytes());
            }
        }
        bridge
            .configure_system_file(
                NativeSystemFile::load(&bytes, &defaults).unwrap(),
                &Names::parse(b"", false).unwrap(),
            )
            .unwrap();
        let names = Names::parse(b"expmode1 expmode2 expmode3 expmode4 expmode5 expcheck1 expcheck2 expcheck3 expcheck4 expcheck5 expmode6", false).unwrap();
        let mut variables = crate::config::configured_variables(
            VariableProfile::Legacy1000,
            &names,
            &Names::parse(b"", false).unwrap(),
        )
        .unwrap();
        for index in 0..11 {
            variables.write_stored(index, 777);
        }
        // Every native getter group is consumed by original postfix assignment
        // on the actual VM worker, not by a test-only direct binding helper.
        let mut records = Vec::new();
        for id in 0..11u32 {
            records.extend([
                8,
                0x66660005,
                0,
                0xa0000000,
                3,
                0x80000064 + id,
                0x80000000 + id,
                0xa000003d,
            ]);
        }
        records.extend([3, 0x66660002, 0x55560005]);
        let mut vm = crate::adapter::NativeAdapter::from_prepared(
            std::sync::Arc::new(program(&records)),
            Machine::with_variables(variables),
            bridge,
        )
        .unwrap();
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while vm.step(&mut runtime.host, Default::default(), 100).unwrap() != VmYield::Halted {
            assert!(std::time::Instant::now() < until);
            std::thread::yield_now();
        }
        let (machine, bridge) = vm.into_idle_state().unwrap();
        let variables = machine.variables().unwrap();
        for id in 0..5 {
            assert_eq!(variables.read_stored(100 + id), -70 + i32::from(id));
        }
        for id in 0..5 {
            assert_eq!(variables.read_stored(105 + id), 120 + i32::from(id));
        }
        assert_eq!(
            variables.read_stored(110),
            777,
            "sixth preference name was never bound by the original legacy constructor"
        );
        assert_eq!(
            variables.read_stored(0),
            777,
            "calculated getter does not overwrite native stored bank"
        );
        assert_eq!(bridge.api_calls, 0);
        assert_eq!(runtime.host.now_ms(), 0);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
    }
    #[test]
    fn savesystemfile_waits_for_real_storage_and_commits_expression_candidate_once_without_clock_or_scene_changes()
     {
        let (_dir, mut runtime) = runtime();
        let mut bridge = bridge();
        let defaults = file();
        bridge
            .configure_system_file(defaults.clone(), &Names::parse(b"", false).unwrap())
            .unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.write_stored(200, 17);
        let mut machine = Machine::with_variables(variables);
        let program = program(&[
            9, 0x66660004, 0, 1, 0xa0000000, 3, 0x800000c8, 0x90000001, 0xa200002b,
        ]);
        let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
            panic!()
        };
        let scene = serde_json::to_value(&runtime.host.engine).unwrap();
        assert!(
            !bridge
                .try_commit(&mut machine, &call, &mut runtime.host)
                .unwrap()
        );
        assert_eq!(machine.pc(), call.pc);
        assert_eq!(machine.variables().unwrap().read_stored(200), 17);
        assert!(bridge.dependency_pending());
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !bridge
            .try_commit(&mut machine, &call, &mut runtime.host)
            .unwrap()
        {
            machine.validate_receipt(&call).unwrap();
            assert!(std::time::Instant::now() < end);
            std::thread::yield_now();
        }
        assert_eq!(machine.pc(), call.next);
        assert_eq!(machine.variables().unwrap().read_stored(200), 18);
        assert!(!bridge.dependency_pending());
        assert_eq!(
            bridge
                .system_file
                .as_ref()
                .unwrap()
                .system_word(92)
                .unwrap(),
            1
        );
        let storage = runtime.host.storage(ClientId(1)).unwrap();
        let path =
            crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, "sysfile.sav")
                .unwrap();
        let bytes =
            crysirror_resource::read_all(storage.open_read(&path).unwrap().as_ref(), 1 << 20)
                .unwrap();
        let loaded = NativeSystemFile::load(&bytes, &defaults).unwrap();
        let mut restored = Variables::fresh(VariableProfile::Legacy1000);
        loaded
            .restore_variables(&mut restored, VariableProfile::Legacy1000)
            .unwrap();
        assert_eq!(restored.read_stored(200), 18);
        assert!(matches!(
            runtime.host.storage(ClientId(2)).unwrap().open_read(&path),
            Err(crysirror_core::Error::NotFound)
        ));
        assert_eq!(bridge.api_calls, 2);
        assert_eq!(runtime.host.now_ms(), 0);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), scene);
        assert!(
            bridge
                .try_commit(&mut machine, &call, &mut runtime.host)
                .is_err(),
            "completed native save cannot execute twice from a stale receipt"
        );
    }
    #[test]
    fn system_save_rejects_unplanned_resource_argument_writes_before_any_dependency() {
        let (_dir, mut runtime) = runtime();
        let mut bridge = bridge();
        bridge
            .configure_system_file(file(), &Names::parse(b"", false).unwrap())
            .unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.mark_write_effect(200, b"layerOffVar0".to_vec());
        variables.write_stored(200, 17);
        let mut machine = Machine::with_variables(variables);
        let program = program(&[
            9, 0x66660004, 0, 1, 0xa0000000, 3, 0x800000c8, 0x90000001, 0xa200002b,
        ]);
        let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
            panic!()
        };
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        assert!(
            bridge
                .try_commit(&mut machine, &call, &mut runtime.host)
                .unwrap_err()
                .to_string()
                .contains("resource-write bindings")
        );
        machine.validate_receipt(&call).unwrap();
        assert_eq!(machine.variables().unwrap().read_stored(200), 17);
        assert_eq!(
            bridge
                .system_file
                .as_ref()
                .unwrap()
                .system_word(92)
                .unwrap(),
            0
        );
        assert_eq!(bridge.api_calls, 0);
        assert!(bridge.pending_system_save.is_none());
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
    }
    #[test]
    fn layer_function_controls_actual_shared_nodes_and_next_canvas_without_dropping_native_flags() {
        let (_dir, mut runtime) = runtime();
        let mut bridge = bridge();
        runtime.host.submit_engine(ClientId(1),1,br#"{"layers":[{"op":"put","layer":{"id":100,"rect":[0,0,4,4],"paint":{"kind":"solid","color":[190,20,30,255]}}}]}"#).unwrap();
        bridge.active_effects.insert(0);
        for (args, visible) in [
            (vec![0x90000000, 0x90000000], false),
            (vec![0x90000000], true),
        ] {
            let mut record = vec![4 + args.len() as u32, 0x66660004, 1, args.len() as u32];
            record.extend(args);
            let program = program(&record);
            let mut machine =
                Machine::with_variables(Variables::fresh(VariableProfile::Legacy1000));
            let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
                panic!()
            };
            bridge
                .commit(&mut machine, &call, &mut runtime.host)
                .unwrap();
            assert_eq!(
                runtime
                    .host
                    .engine
                    .layers
                    .get(ClientId(1), 100)
                    .unwrap()
                    .visible,
                visible
            );
            assert_eq!(bridge.native_layer_off & 1, u32::from(!visible));
            let canvas = bridge
                .prepare_frame_canvas(&mut runtime.host, None)
                .unwrap();
            let surface = runtime
                .host
                .engine
                .canvases
                .surface(ClientId(1), canvas)
                .unwrap();
            let captured = surface.nodes.iter().find_map(|node| match node {
                crysirror_scene::canvas::CanvasNode::Draw { layer } => match &layer.paint {
                    crysirror_scene::layer::Paint::Snapshot { layers, .. } => Some(layers),
                    _ => None,
                },
                _ => None,
            });
            assert_eq!(captured.is_some(), visible);
            if let Some(layers) = captured {
                assert!(layers.iter().any(|node| node.id == 100
                    && node.visible
                    && matches!(
                        node.paint,
                        crysirror_scene::layer::Paint::Solid {
                            color: [190, 20, 30, 255]
                        }
                    )));
            }
            assert_eq!(machine.pc(), call.next);
        }
        assert_eq!(runtime.host.now_ms(), 0);
    }
    #[test]
    fn result_extension_is_not_acknowledged_as_an_ordinary_system_file_save() {
        let (_dir, mut runtime) = runtime();
        let mut bridge = bridge();
        bridge
            .configure_system_file(file(), &Names::parse(b"resultFlag 1", false).unwrap())
            .unwrap();
        let program = program(&[4, 0x66660004, 0, 0]);
        let mut machine = Machine::with_variables(Variables::fresh(VariableProfile::Legacy1000));
        let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
            panic!()
        };
        assert!(
            bridge
                .try_commit(&mut machine, &call, &mut runtime.host)
                .unwrap_err()
                .to_string()
                .contains("ResultData")
        );
        machine.validate_receipt(&call).unwrap();
        assert_eq!(
            bridge
                .system_file
                .as_ref()
                .unwrap()
                .system_word(92)
                .unwrap(),
            0
        );
        assert!(bridge.pending_system_save.is_none());
        assert_eq!(bridge.api_calls, 0);
    }
}

#[cfg(test)]
mod actor_clock_tests {
    use super::*;
    fn bridge() -> Bridge {
        Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [16, 16],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap()
    }
    #[test]
    fn static_native_wait_advances_passive_saved_counts_through_actual_zero_lead_clock() {
        let mut b = bridge();
        b.passive_counts.insert(3, (7, 99999));
        for (now, count) in [(0, 7), (50, 8), (550, 9), (650, 11), (650, 12)] {
            b.advance_static_actor_clock(now).unwrap();
            assert_eq!(b.effect_counter(3), Some((count, 99999)));
        }
        let before = (b.actor_clock.clone(), b.actor_clock_at, b.effect_counter(3));
        assert!(b.advance_static_actor_clock(649).is_err());
        assert_eq!(
            (b.actor_clock.clone(), b.actor_clock_at, b.effect_counter(3)),
            before
        );
    }
    #[test]
    fn original_no_skip_frame_preference_restricts_actual_actor_work_without_changing_saved_bytes()
    {
        use crate::native_system_file::{NativeSystemFile, SystemLimits, SystemNames, SystemStamp};
        let defaults = NativeSystemFile::defaults(
            SystemLimits::new(2, 1, 1),
            SystemStamp([2026, 11, 1, 0, 10, 20, 30, 400]),
            &SystemNames {
                font: b"fixture".to_vec(),
                surname: vec![],
                given_name: vec![],
            },
        )
        .unwrap();
        let mut bytes = defaults.encode().unwrap().to_vec();
        bytes[64 + 152..64 + 156].copy_from_slice(&1i32.to_le_bytes());
        let loaded = NativeSystemFile::load(&bytes, &defaults).unwrap();
        let mut b = bridge();
        b.configure_system_file(
            loaded,
            &crate::config::Names::parse(b"maxSkipFrame 3 skipAdjustFrame 10", false).unwrap(),
        )
        .unwrap();
        b.passive_counts.insert(3, (0, 99999));
        b.advance_static_actor_clock(0).unwrap();
        b.advance_static_actor_clock(500).unwrap();
        b.advance_static_actor_clock(1000).unwrap();
        assert_eq!(b.effect_counter(3), Some((2, 99999)));
        assert_eq!(b.actor_clock.maximum_steps, 1);
        assert_eq!(
            b.system_file.as_ref().unwrap().encode().unwrap().as_ref(),
            bytes
        );
    }
    #[test]
    fn saved_screen_layer_cannot_be_silently_moved_under_the_restored_text() {
        let mut b = bridge();
        b.active_effects.insert(15);
        assert!(b.validate_saved_wait_effects(true, false).is_err());
        b.active_effects.remove(&15);
        b.active_effects.insert(3);
        b.fill_effects.insert(3, [1, 2, 3]);
        assert!(b.validate_saved_wait_effects(true, false).is_ok());
    }
}
