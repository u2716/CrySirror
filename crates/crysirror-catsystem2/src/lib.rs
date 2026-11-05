// SPDX-License-Identifier: MPL-2.0
//! Original CatSystem2 script policy over CrySirror capability services.
//! No original engine, executable, renderer, mixer or game content is linked.
//!
//! Implementation map:
//! - `adapter.rs`: KCS syscall dispatch, scoped configuration and completion.
//! - `fes_runtime.rs` / `fes_natives.rs`: FES task and command semantics.
//! - `fes_object_resources.rs`: image policy mapped to shared Image/Canvas/Layer.
//! - `native_image_resource.rs`: HG3 selection and original geometry metadata.
//! - `fes_panel_parts.rs` / `fes_panel_render.rs`: configured and live PANEL parts.
//! - `native_log.rs` / `fes_text.rs`: message history and native text policy.
//! - `native_m2.rs` / `native_classes/emote_messages.rs`: E-mote native policy.
//! - `native_menu.rs`: native menu IDs mapped to shared UI dialogs.
//! - `native_classes/view3d.rs`: reviewed factory and native-save receiver policy.
//!
//! Native implementation evidence belongs beside these operations. Reusable
//! drawing, text, resource and audio contracts belong to their engine crates.
#![forbid(unsafe_code)]
mod adapter;
pub mod kcs;
pub mod motion;
mod motion_math;
mod motion_set;
mod native_menu;
mod native_values;
mod native_interpreter;
mod sound_definition;
mod native_sound_policy;
mod native_save;
mod native_store;
mod native_math;
mod native_log;
mod native_misc;
mod native_classes;
mod image_list;
mod fes_task;
mod fes_program;
mod fes_declarations;
mod fes_object_values;
mod native_object_pose;
mod fes_references;
mod fes_key_blocks;
mod fes_macros;
mod fes_loading;
mod fes_children;
mod fes_natives;
mod fes_image_methods;
mod fes_bg_table;
mod fes_panel_parts;
mod fes_sound_tags;
mod fes_save;
#[cfg(feature = "runtime")]
mod fes_file_list;
#[cfg(feature = "runtime")]
mod fes_save_panel;
mod fes_sound_table;
mod fes_button;
mod fes_file_probe;
mod fes_civil_time;
mod fes_flow;
mod fes_timing;
mod fes_runtime;
mod fes_strings;
mod fes_values;
pub mod scene;
pub mod startup;
mod task_policy;
#[cfg(feature = "runtime")]
pub use adapter::register;
pub use adapter::{CatSystem2Vm, Config};
pub use startup::{parse_startup, Startup};

mod loop_policy;

mod fes_text;
#[cfg(feature = "runtime")]
mod native_text_coverage;
mod native_plane;
mod native_scene;
mod native_strings;
mod native_motion;
#[cfg(feature="runtime")]
mod native_kcs_sound;
#[cfg(feature="runtime")]
mod native_m2;
mod native_scalar;
mod native_messages;
mod native_string_map;
mod native_settings;
mod native_input;
mod native_platform;
mod native_task_result;

mod texture_cache;
#[cfg(feature = "runtime")]
mod native_image_resource;
mod native_resource_paths;
mod native_image_publication;
#[cfg(feature="runtime")]
mod native_surface;
#[cfg(feature = "runtime")]
mod native_surface_ops;
#[cfg(feature = "runtime")]
mod native_movie;
mod native_class_state;
mod fes_object_bank;

#[cfg(feature = "runtime")]
mod fes_object_resources;
#[cfg(feature = "runtime")]
mod fes_panel_render;
#[cfg(feature = "runtime")]
mod fes_live_frames;
#[cfg(feature = "runtime")]
mod fes_base_image;
#[cfg(feature="runtime")]
mod fes_live_motions;
#[cfg(feature="runtime")]
mod fes_live_sound;
#[cfg(feature="runtime")]
mod native_sound_fades;
#[cfg(feature="runtime")]
mod native_sound_loops;
mod fes_sound_commands;

mod fes_alias_values;
mod fes_object_natives;
mod fes_random;
