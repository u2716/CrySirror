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
pub(crate) mod input_task;
mod plane_messages;
#[cfg(feature = "runtime")]
mod effect_tasks;
mod text_sound_tasks;
mod shader_tasks;
mod emote_messages;
mod containers;
mod image_ops;
pub(crate) mod database;
mod subsystems;
pub(crate) mod text_window;
mod name_table;
mod host_misc;
mod export_task;
pub(crate) mod cg_register;
mod scene_log;
mod plane_effects;
mod scene_records;
mod render_settings;
mod tables_3d;
pub(crate) mod view3d;
#[cfg(feature = "runtime")]
pub(crate) mod particles;
#[cfg(feature="runtime")]
mod particle_plugins;
#[cfg(feature="runtime")]
mod particle_rain;
mod particle_dash;
mod particle_effects;
mod particle_cmn;

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
    /// VM+180 string variable bank.
    pub strings: &'a mut crate::native_values::StringVariables,
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
    #[cfg(feature = "runtime")]
    pub effect_tasks: effect_tasks::State,
    pub text_sound_tasks: text_sound_tasks::State,
    pub shader_tasks: shader_tasks::State,
    pub emote_messages: emote_messages::State,
    pub containers: containers::State,
    pub image_ops: image_ops::State,
    pub subsystems: subsystems::State,
    pub text_window: text_window::State,
    pub name_table: name_table::State,
    pub host_misc: host_misc::State,
    pub export_task: export_task::State,
    pub cg_register: cg_register::State,
    pub scene_log: scene_log::State,
    pub plane_effects: plane_effects::State,
    pub scene_records: scene_records::State,
    pub render_settings: render_settings::State,
    pub tables_3d: tables_3d::State,
    #[cfg(feature = "runtime")]
    pub particles: particles::State,
}

/// True when a family implements this native.
pub(crate) fn claims(id: u32) -> bool {
    os_window::IDS.contains(&id)
        || input_task::IDS.contains(&id)
        || plane_messages::IDS.contains(&id)
        || text_sound_tasks::IDS.contains(&id)
        || shader_tasks::IDS.contains(&id)
        || emote_messages::IDS.contains(&id)
        || containers::IDS.contains(&id)
        || image_ops::IDS.contains(&id)
        || database::IDS.contains(&id)
        || subsystems::IDS.contains(&id)
        || text_window::IDS.contains(&id)
        || name_table::IDS.contains(&id)
        || host_misc::IDS.contains(&id)
        || export_task::IDS.contains(&id)
        || cg_register::IDS.contains(&id)
        || scene_log::IDS.contains(&id)
        || plane_effects::IDS.contains(&id)
        || scene_records::IDS.contains(&id)
        || render_settings::IDS.contains(&id)
        || tables_3d::IDS.contains(&id)
        || runtime_claims(id)
}

#[cfg(feature = "runtime")]
fn runtime_claims(id: u32) -> bool {
    particles::IDS.contains(&id) || effect_tasks::IDS.contains(&id)
}
#[cfg(not(feature = "runtime"))]
fn runtime_claims(_id: u32) -> bool {
    false
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
    #[cfg(feature = "runtime")]
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
    if database::IDS.contains(&id) {
        return database::handle(cx);
    }
    if subsystems::IDS.contains(&id) {
        return subsystems::handle(cx);
    }
    if text_window::IDS.contains(&id) {
        return text_window::handle(cx);
    }
    if name_table::IDS.contains(&id) {
        return name_table::handle(cx);
    }
    if scene_records::IDS.contains(&id) {
        return scene_records::handle(cx);
    }
    if render_settings::IDS.contains(&id) {
        return render_settings::handle(cx);
    }
    if plane_effects::IDS.contains(&id) {
        return plane_effects::handle(cx);
    }
    if scene_log::IDS.contains(&id) {
        return scene_log::handle(cx);
    }
    if cg_register::IDS.contains(&id) {
        return cg_register::handle(cx);
    }
    if export_task::IDS.contains(&id) {
        return export_task::handle(cx);
    }
    if host_misc::IDS.contains(&id) {
        return host_misc::handle(cx);
    }
    if tables_3d::IDS.contains(&id) {
        return tables_3d::handle(cx);
    }
    #[cfg(feature = "runtime")]
    if particles::IDS.contains(&id) {
        return particles::handle(cx);
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
    plane_effects::tick(host, client, tasks, &mut state.plane_effects)?;
    input_task::tick(host, client, tasks, &mut state.input_task, now_ms)?;
    #[cfg(feature = "runtime")]
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
    #[cfg(feature = "runtime")]
    effect_tasks::retire(host, client, &mut state.effect_tasks, id)?;
    shader_tasks::retire(host, client, &mut state.shader_tasks, id)?;
    text_sound_tasks::retire(host, client, &mut state.text_sound_tasks, id)?;
    plane_messages::retire(host, client, &mut state.plane_messages, id)?;
    plane_effects::retire(host, client, &mut state.plane_effects, id)?;
    text_window::retire(host, client, &mut state.text_window, id)?;
    #[cfg(feature = "runtime")]
    particles::retire(host, client, &mut state.particles, id)?;
    Ok(())
}
