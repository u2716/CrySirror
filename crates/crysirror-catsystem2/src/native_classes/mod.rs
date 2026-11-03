// SPDX-License-Identifier: MPL-2.0
//! Original KCS natives grouped by the native task class or subsystem behind
//! them (dispatch table A2DD38; handler sources in the ignored static evidence).
//! Each family keeps only original semantic state and policy and reaches pixels,
//! audio, input, files and platform effects through shared CrySirror services.
//! A family claims a native id only once it implements that native's effect.
use crate::kcs::{Machine, NativeCall};
use anyhow::Result;
use crysirror_core::{service::HostServices, state::ClientId};

mod os_window;
mod input_task;
mod plane_messages;
mod effect_tasks;
mod text_sound_tasks;
mod shader_tasks;
mod emote_messages;
mod containers;
mod image_ops;
mod subsystems;
pub(crate) mod text_window;

/// Everything one native call may touch. Fields are disjoint adapter borrows.
pub(crate) struct Ctx<'a> {
    pub machine: &'a mut Machine,
    pub host: &'a mut dyn HostServices,
    pub client: ClientId,
    pub tasks: &'a mut crate::task_policy::NativeTaskPolicy,
    pub state: &'a mut State,
    /// Original startup.xml virtual canvas.
    pub canvas: [u32; 2],
    pub call: &'a NativeCall,
    /// Bounded worker pool for resource preparation.
    pub pool: Option<&'a crysirror_tasks::TaskPool>,
    /// Original configuration and key bindings, once the native settings loaded.
    pub settings: Option<&'a mut crate::native_settings::NativeSettings>,
    /// KCS-owned native handles (images from 710, scenes, motion sets).
    pub handles: &'a mut crate::native_scene::Handles,
    /// FES flag/expression interpreter and per-player local banks.
    pub interpreter: Option<&'a mut crate::native_interpreter::Interpreter>,
    /// CSS save containers (78B600 family) and their handle counter.
    pub saves: &'a mut std::collections::BTreeMap<u32, crate::native_save::Container>,
    pub next_save: &'a mut u32,
    /// Licensed font alias of this project (adapter Config.font).
    pub font: &'a str,
    pub input: &'a mut crate::native_input::InputState,
    /// Adapter frame counter (owner ticks).
    pub ticks: u64,
    #[cfg(feature = "runtime")]
    pub m2: &'a mut crate::native_m2::Planes,
    #[cfg(feature = "runtime")]
    pub movies: &'a mut crate::native_movie::Movies,
    #[cfg(feature = "runtime")]
    pub kcs_sound: &'a mut crate::native_kcs_sound::KcsSound,
}

/// 6A49D0: optional sign, then "0x"/"0X" hex or decimal digits; a '.' right
/// after the digits fails. Answers (value, bytes consumed).
pub(crate) fn parse_number(bytes: &[u8]) -> Option<(i32, usize)> {
    text_window::number(bytes)
}

/// Per-family native state, owned by the adapter.
#[derive(Default)]
pub(crate) struct State {
    pub os_window: os_window::State,
    pub input_task: input_task::State,
    pub plane_messages: plane_messages::State,
    pub effect_tasks: effect_tasks::State,
    pub text_sound_tasks: text_sound_tasks::State,
    pub shader_tasks: shader_tasks::State,
    pub emote_messages: emote_messages::State,
    pub containers: containers::State,
    pub image_ops: image_ops::State,
    pub subsystems: subsystems::State,
    pub text_window: text_window::State,
}

/// True when a family implements this native.
pub(crate) fn claims(id: u32) -> bool {
    os_window::IDS.contains(&id)
        || input_task::IDS.contains(&id)
        || plane_messages::IDS.contains(&id)
        || effect_tasks::IDS.contains(&id)
        || text_sound_tasks::IDS.contains(&id)
        || shader_tasks::IDS.contains(&id)
        || emote_messages::IDS.contains(&id)
        || containers::IDS.contains(&id)
        || image_ops::IDS.contains(&id)
        || subsystems::IDS.contains(&id)
        || text_window::IDS.contains(&id)
}

/// Run one claimed native. The family acknowledges or commits the call itself.
/// `Ok(false)` leaves the call pending (a worker is still preparing its result);
/// the adapter yields the frame and offers the same call again.
pub(crate) fn dispatch(cx: &mut Ctx) -> Result<bool> {
    let id = cx.call.id;
    if input_task::IDS.contains(&id) {
        return input_task::handle(cx);
    }
    if os_window::IDS.contains(&id) {
        return os_window::handle(cx);
    }
    if plane_messages::IDS.contains(&id) {
        return plane_messages::handle(cx);
    }
    if effect_tasks::IDS.contains(&id) {
        return effect_tasks::handle(cx);
    }
    if text_sound_tasks::IDS.contains(&id) {
        return text_sound_tasks::handle(cx);
    }
    if shader_tasks::IDS.contains(&id) {
        return shader_tasks::handle(cx);
    }
    if emote_messages::IDS.contains(&id) {
        return emote_messages::handle(cx);
    }
    if containers::IDS.contains(&id) {
        return containers::handle(cx);
    }
    if image_ops::IDS.contains(&id) {
        return image_ops::handle(cx);
    }
    if subsystems::IDS.contains(&id) {
        return subsystems::handle(cx);
    }
    if text_window::IDS.contains(&id) {
        return text_window::handle(cx);
    }
    anyhow::bail!("CatSystem2 native {id} is not claimed by a native family")
}

/// Native 9 asked the game window to close (0x433 → 0x410).
pub(crate) fn take_halt(state: &mut State) -> bool {
    core::mem::take(&mut state.os_window.halt)
}

/// Once per owner frame, before the main script runs (after E-mote ticking).
pub(crate) fn tick(
    host: &mut dyn HostServices,
    client: ClientId,
    tasks: &mut crate::task_policy::NativeTaskPolicy,
    state: &mut State,
    now_ms: u64,
) -> Result<()> {
    os_window::tick(host, client, &mut state.os_window)?;
    plane_messages::tick(host, client, tasks, &mut state.plane_messages, now_ms)?;
    input_task::tick(host, client, tasks, &mut state.input_task, now_ms)?;
    effect_tasks::tick(host, client, tasks, &mut state.effect_tasks, now_ms)?;
    shader_tasks::tick(host, client, tasks, &mut state.shader_tasks, now_ms)?;
    text_sound_tasks::tick(host, client, tasks, &mut state.text_sound_tasks, now_ms)?;
    subsystems::tick(host, client, tasks, &mut state.subsystems, now_ms)?;
    Ok(())
}

/// Native 67 destroyed task `id`; release whatever a family published for it.
pub(crate) fn retire(
    host: &mut dyn HostServices,
    client: ClientId,
    state: &mut State,
    id: u32,
) -> Result<()> {
    effect_tasks::retire(host, client, &mut state.effect_tasks, id)?;
    shader_tasks::retire(host, client, &mut state.shader_tasks, id)?;
    text_sound_tasks::retire(host, client, &mut state.text_sound_tasks, id)?;
    plane_messages::retire(host, client, &mut state.plane_messages, id)?;
    text_window::retire(host, client, &mut state.text_window, id)?;
    Ok(())
}
