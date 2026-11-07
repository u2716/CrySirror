// SPDX-License-Identifier: MPL-2.0
//! Device-independent, bounded VFX authoring, compilation and reference simulation.
//! No UE implementation, renderer, window, mixer or executable code is embedded.
#![forbid(unsafe_code)]
pub mod falling_atlas;
pub mod orbit_atlas;
mod schema;
pub mod screen;
/// Device-free screen/layer shake curves (pattern, oscillation, circle, zoom).
pub mod shake;
mod simulation;
pub mod spatial;
pub use schema::*;
pub use simulation::*;

/// Externally clocked fixed-point particle fields with caller-owned randomness.
pub mod fixed_particle;

pub mod tile_mask;

pub mod drifting_atlas;

///Integer atlas pools independent of resources, VM and renderer.
pub mod cyclic_atlas;
