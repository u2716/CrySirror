// SPDX-License-Identifier: MPL-2.0
pub mod codes;
pub(crate) mod cgtable;
pub mod counter;
pub mod excall;
pub mod frame_action;
pub mod frame_action_ch;
pub mod global;
pub mod input;
pub mod int_event;
pub mod int_list;
pub mod key;
pub mod math;
pub mod mouse;
pub mod mov;
mod object_event;
mod passive_object;
mod object_metadata;
#[cfg(test)]
mod object_lifetime_tests;
pub mod pcmch;
pub mod prop_access;
pub mod script;
mod script_native;
#[cfg(test)]
mod script_policy;
mod object_flags;
mod object_frame_action;
pub mod stage;
mod stage_lifecycle;
pub mod str_list;
pub mod syscom;
pub(crate) mod syscom_original;
mod sound_config;
mod cursor_preferences;
mod font_preferences;
mod syscom_flags;
mod syscom_preferences;
pub mod system;
pub mod timewait;
pub fn dispatch_form(
    ctx: &mut super::CommandContext,
    id: u32,
    args: &[super::Value],
) -> anyhow::Result<bool> {
    super::dispatch_form_code(ctx, id, args)
}
