// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! WillPlus/AdvHD native data frontend. Shared Resource owns archives and I/O;
//! the adapter owns bytecode layouts and eventual native execution semantics.
#![forbid(unsafe_code)]
pub mod adapter;
pub mod bootstrap;
pub mod layers;
pub mod log;
pub mod message;
pub mod save;
pub mod system;
pub mod vm;
pub mod program;
mod tables;
