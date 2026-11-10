// SPDX-License-Identifier: MPL-2.0
//! Shared desktop runtime. Native engine adapters are registered explicitly.
#![forbid(unsafe_code)]
pub mod assets;
pub mod csv;
pub mod frame;
pub mod font_resources;
pub mod media;
pub mod model;
pub mod physics;
pub mod runtime;
pub mod script;
pub mod structured_text;
#[cfg(not(target_arch = "wasm32"))]
pub mod vfs;
pub mod storage;
pub mod vfx;
mod vfx_query;
pub mod vector;
pub use model::Project;
pub use runtime::{AdapterRegistry, Runtime};

mod image_processing;
mod image_encoding;
mod audio_replacement;

pub mod text_storage;
pub mod text_input;

pub mod published_images;

mod civil_time;
mod session_seed;

mod pattern_resources;
pub mod canvas_images;
mod detached_layers;

mod media_image;

mod scene_checkpoint;
